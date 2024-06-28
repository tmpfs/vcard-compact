//! Parse vCards based on [RFC6350](https://www.rfc-editor.org/rfc/rfc6350).

use logos::{Lexer, Logos};
use std::{borrow::Cow, ops::Range};
use uriparse::uri::URI as Uri;

#[cfg(feature = "language-tags")]
use language_tags::LanguageTag;

#[cfg(feature = "mime")]
use mime::Mime;

use crate::{
    error::LexError, escape_control, helper::*, name::*, parameter::*,
    property::*, unescape_value, Error, Result, Vcard,
};

type LexResult<T> = std::result::Result<T, LexError>;

#[derive(Logos, Debug, PartialEq)]
#[logos(error = LexError)]
pub(crate) enum Token {
    #[regex("(?i:BEGIN:VCARD)")]
    Begin,

    #[regex("(?i:VERSION:[3-4]\\.0)")]
    Version,

    // Special case shared between property and parameter
    #[token("TZ")]
    TimeZone,

    // Special case shared between property and parameter
    #[token("GEO")]
    Geo,

    #[regex("(?i:([a-z0-9-]+\\.)?(SOURCE|KIND|FN|N|NICKNAME|PHOTO|BDAY|ANNIVERSARY|GENDER|ADR|TEL|EMAIL|IMPP|LANG|TITLE|ROLE|LOGO|ORG|MEMBER|RELATED|CATEGORIES|NOTE|PRODID|REV|SOUND|UID|CLIENTPIDMAP|URL|KEY|FBURL|CALADRURI|CALURI|XML|VERSION|(X-[a-z0-9-]+)))")]
    PropertyName,

    #[regex("(?i:x-[a-z0-9-]+)")]
    ExtensionName,

    #[token(";")]
    ParameterDelimiter,

    #[token("\"")]
    DoubleQuote,

    #[regex("(?i:LANGUAGE|VALUE|PREF|ALTID|PID|TYPE|MEDIATYPE|CALSCALE|SORT-AS|CHARSET|LABEL|ENCODING)")]
    ParameterKey,

    #[token("=")]
    ValueDelimiter,

    #[token(":")]
    PropertyDelimiter,

    #[regex("\\r?\\n( |\\t)")]
    FoldedLine,

    #[token("\\,")]
    EscapedComma,

    #[token("\\;")]
    EscapedSemiColon,

    #[token("\\\\")]
    EscapedBackSlash,

    #[regex("(?i:\\\\n)")]
    EscapedNewLine,

    #[regex("\\r?\\n", priority = 3)]
    NewLine,

    #[regex("[[:blank:]]", priority = 2)]
    WhiteSpace,

    #[regex("[[:cntrl:]]", priority = 1)]
    Control,

    #[regex("(?i:END:VCARD)")]
    End,

    #[regex("[\u{00}-\u{7F}]", priority = 0)]
    Text,
}

/// Parses vCards from a string.
pub(crate) struct VcardParser<'s> {
    strict: bool,
    pub(crate) source: &'s str,
}

impl<'s> VcardParser<'s> {
    /// Create a new parser.
    pub fn new(source: &'s str, strict: bool) -> Self {
        Self { source, strict }
    }

    /// Parse a UTF-8 encoded string into a list of vCards.
    pub(crate) fn parse(&self) -> Result<Vec<Vcard>> {
        let mut cards = Vec::new();
        let mut lex = self.lexer();

        while let Some(first) = lex.next() {
            // Allow leading newlines and newlines between
            // vCard definitions
            if first == Ok(Token::NewLine) {
                continue;
            }

            let (card, _) = self.parse_one(&mut lex, Some(first))?;
            card.validate()?;
            cards.push(card);
        }

        if cards.is_empty() {
            return Err(Error::TokenExpected);
        }

        Ok(cards)
    }

    /// Get a lexer for the current source.
    pub(crate) fn lexer(&self) -> Lexer<'s, Token> {
        Token::lexer(self.source)
    }

    /// Parse a single vCard.
    pub(crate) fn parse_one(
        &self,
        lex: &mut Lexer<'_, Token>,
        first: Option<LexResult<Token>>,
    ) -> Result<(Vcard, Range<usize>)> {
        self.assert_token(first.as_ref(), &[Token::Begin])?;
        self.assert_token(lex.next().as_ref(), &[Token::NewLine])?;

        self.assert_token(lex.next().as_ref(), &[Token::Version])?;
        self.assert_token(lex.next().as_ref(), &[Token::NewLine])?;

        let mut card: Vcard = Default::default();

        self.parse_properties(lex, &mut card)?;

        Ok((card, lex.span()))
    }

    /// Parse the properties of a vCard.
    fn parse_properties(
        &self,
        lex: &mut Lexer<'_, Token>,
        card: &mut Vcard,
    ) -> Result<()> {
        while let Some(first) = lex.next() {
            if first == Ok(Token::End) {
                break;
            }
            if let Ok(Token::Version) = first {
                return Err(Error::VersionMisplaced);
            }

            self.assert_token(
                Some(&first),
                &[
                    Token::PropertyName,
                    Token::ExtensionName,
                    Token::TimeZone,
                    Token::Geo,
                ],
            )?;

            if let Err(e) = self.parse_property(lex, first, card) {
                if self.strict {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Parse a single property.
    fn parse_property(
        &self,
        lex: &mut Lexer<'_, Token>,
        token: LexResult<Token>,
        card: &mut Vcard,
    ) -> Result<()> {
        let mut group: Option<String> = None;
        let mut name = lex.slice();

        let period = name.find('.');
        if let Some(pos) = period {
            let group_name = &name[0..pos];
            group = Some(group_name.to_string());
            name = &name[pos + 1..];
        }

        let delimiter = lex.next();

        if let Some(delimiter) = delimiter {
            if delimiter == Ok(Token::ParameterDelimiter) {
                let parameters = self.parse_parameters(lex, name)?;
                self.parse_property_by_name(
                    lex,
                    token,
                    card,
                    name,
                    Some(parameters),
                    group,
                )?;
            } else if delimiter == Ok(Token::PropertyDelimiter) {
                self.parse_property_by_name(
                    lex, token, card, name, None, group,
                )?;
            } else {
                return Err(Error::DelimiterExpected);
            }
        } else {
            return Err(Error::TokenExpected);
        }

        Ok(())
    }

    fn add_extension_parameter(
        &self,
        parameter_name: &str,
        value: String,
        params: &mut Parameters,
    ) {
        let values =
            value.split(',').map(|s| s.to_owned()).collect::<Vec<_>>();
        let x_param = (parameter_name.to_owned(), values);
        if let Some(extensions) = params.extensions.as_mut() {
            extensions.push(x_param);
        } else {
            params.extensions = Some(vec![x_param]);
        }
    }

    /// Parse property parameters.
    fn parse_parameters(
        &self,
        lex: &mut Lexer<'_, Token>,
        name: &str,
    ) -> Result<Parameters> {
        let property_upper_name = name.to_uppercase();
        let mut params: Parameters = Default::default();
        let mut next: Option<LexResult<Token>> = lex.next();

        while let Some(token) = next.take() {
            if token == Ok(Token::ParameterKey)
                || token == Ok(Token::ExtensionName)
                || token == Ok(Token::TimeZone)
                || token == Ok(Token::Geo)
            {
                let source = lex.source();
                let span = lex.span();
                let parameter_name = &source[span.start..span.end];
                let upper_name = parameter_name.to_uppercase();

                self.assert_token(
                    lex.next().as_ref(),
                    &[Token::ValueDelimiter],
                )?;

                let (value, next_token, quoted) =
                    self.parse_parameter_value(lex)?;

                if token == Ok(Token::ExtensionName) {
                    self.add_extension_parameter(
                        parameter_name,
                        value,
                        &mut params,
                    );
                } else {
                    match &upper_name[..] {
                        LANGUAGE => {
                            let tag = parse_language_tag(Cow::Owned(value))?;
                            params.language = Some(tag);
                        }
                        VALUE => {
                            let value: ValueType = value.parse()?;
                            params.value = Some(value);
                        }
                        PREF => {
                            let value: u8 = value.parse()?;
                            if !(1..=100).contains(&value) {
                                return Err(Error::PrefOutOfRange(value));
                            }
                            params.pref = Some(value);
                        }
                        ALTID => {
                            params.alt_id = Some(value);
                        }
                        PID => {
                            let mut pids: Vec<Pid> = Vec::new();
                            let values = value.split(',');
                            for value in values {
                                pids.push(value.parse()?);
                            }
                            params.pid = Some(pids);
                        }
                        TYPE => {
                            // Check this parameter is allowed
                            if !TYPE_PROPERTIES
                                .contains(&&property_upper_name[..])
                                && !property_upper_name.starts_with("X-")
                            {
                                return Err(Error::TypeParameter(
                                    property_upper_name,
                                ));
                            }

                            let mut type_params: Vec<TypeParameter> =
                                Vec::new();

                            for val in value.split(',') {
                                let param: TypeParameter = val.parse()?;
                                type_params.push(param);
                            }

                            if let Some(types) = params.types.as_mut() {
                                types.append(&mut type_params);
                            } else {
                                params.types = Some(type_params);
                            }
                        }
                        MEDIATYPE => {
                            parse_media_type(value, &mut params)?;
                        }
                        CALSCALE => {
                            params.calscale = Some(value);
                        }
                        SORT_AS => {
                            let sort_values = value
                                .split(',')
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>();
                            params.sort_as = Some(sort_values);
                        }
                        GEO => {
                            if !quoted {
                                return Err(Error::NotQuoted(
                                    property_upper_name,
                                ));
                            }
                            let geo = Uri::try_from(&value[..])?.into_owned();
                            params.geo = Some(geo);
                        }
                        TZ => {
                            if quoted {
                                let value =
                                    Uri::try_from(&value[..])?.into_owned();
                                params.timezone =
                                    Some(TimeZoneParameter::Uri(value));
                            } else {
                                match parse_utc_offset(&value) {
                                    Ok(offset) => {
                                        params.timezone = Some(
                                            TimeZoneParameter::UtcOffset(
                                                offset,
                                            ),
                                        );
                                    }
                                    Err(_) => {
                                        params.timezone = Some(
                                            TimeZoneParameter::Text(value),
                                        );
                                    }
                                }
                            }
                        }
                        CHARSET => {
                            // Ignore CHARSET=UTF-8 for compatibility with software that
                            // unnecessarily (and in spite of RFC 6350) adds this parameter.
                            if value != "UTF-8" {
                                return Err(Error::CharsetParameter(value));
                            }
                        }
                        LABEL => {
                            if property_upper_name != ADR {
                                return Err(Error::InvalidLabel(
                                    property_upper_name,
                                ));
                            }
                            params.label = Some(value);
                        }
                        ENCODING => {
                            self.add_extension_parameter(
                                parameter_name,
                                value,
                                &mut params,
                            );
                        }
                        _ => {
                            return Err(Error::UnknownParameter(
                                parameter_name.to_string(),
                            ))
                        }
                    }
                }

                if next_token == Ok(Token::PropertyDelimiter) {
                    break;
                } else if next_token == Ok(Token::ParameterKey) {
                    next = Some(next_token);
                } else {
                    next = lex.next();
                }
            } else {
                return Err(Error::UnknownParameter(lex.slice().to_string()));
            }
        }
        Ok(params)
    }

    /// Parse the raw value for a property parameter.
    fn parse_parameter_value(
        &self,
        lex: &mut Lexer<'_, Token>,
    ) -> Result<(String, LexResult<Token>, bool)> {
        let mut first_range: Option<Range<usize>> = None;
        let mut quoted = false;
        let mut is_folded_or_escaped = false;

        while let Some(mut token) = lex.next() {
            let span = lex.span();

            if token == Ok(Token::Control) {
                return Err(Error::ControlCharacter(escape_control(
                    lex.slice(),
                )));
            }

            if token == Ok(Token::FoldedLine)
                || token == Ok(Token::EscapedNewLine)
                || token == Ok(Token::EscapedComma)
                || token == Ok(Token::EscapedBackSlash)
            {
                is_folded_or_escaped = true;
            }

            let completed = if first_range.is_some() && quoted {
                token == Ok(Token::DoubleQuote)
            } else {
                token == Ok(Token::PropertyDelimiter)
                    || token == Ok(Token::ParameterDelimiter)
                //|| token == Token::ParameterKey
            };

            if first_range.is_none() {
                first_range = Some(span.clone());
                if token == Ok(Token::DoubleQuote) {
                    quoted = true;
                }
            }

            if completed {
                let source = lex.source();
                let begin = first_range.unwrap().start;
                let end = span.start;
                let mut value = &source[begin..end];

                // Remove double quotes if necessary
                if value.len() >= 2 && quoted {
                    value = &source[begin + 1..end];
                }

                // Must consumer the next token
                if quoted {
                    token = if let Some(Ok(token)) = lex.next() {
                        if token != Token::PropertyDelimiter
                            && token != Token::ParameterDelimiter
                        {
                            return Err(Error::DelimiterExpected);
                        }
                        Ok(token)
                    } else {
                        return Err(Error::TokenExpected);
                    };
                }

                let value = if is_folded_or_escaped {
                    unescape_value(value)
                } else {
                    value.to_string()
                };

                return Ok((value, token, quoted));
            }
        }
        Err(Error::TokenExpected)
    }

    /// Parse a property by name.
    fn parse_property_by_name(
        &self,
        lex: &mut Lexer<'_, Token>,
        token: LexResult<Token>,
        card: &mut Vcard,
        name: &str,
        parameters: Option<Parameters>,
        group: Option<String>,
    ) -> Result<()> {
        let value = self.parse_property_value(lex)?;

        let upper_name = name.to_uppercase();

        if token == Ok(Token::ExtensionName) || upper_name.starts_with("X-") {
            self.parse_extension_property_by_name(
                card, name, value, parameters, group,
            )?;
            return Ok(());
        }

        match &upper_name[..] {
            // General properties
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.1
            SOURCE => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.source.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            KIND => {
                if card.kind.is_some() {
                    return Err(Error::OnlyOnce(upper_name));
                }
                let value: Kind = value.as_ref().parse()?;
                card.kind = Some(KindProperty {
                    value,
                    parameters,
                    group,
                });
            }
            XML => {
                card.xml.push(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                });
            }
            // Identification properties
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.2
            FN => {
                card.formatted_name.push(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                });
            }
            N => {
                if card.name.is_some() {
                    return Err(Error::OnlyOnce(upper_name));
                }
                let value = value
                    .as_ref()
                    .split(';')
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                card.name = Some(TextListProperty {
                    value,
                    parameters,
                    group,
                    delimiter: TextListDelimiter::SemiColon,
                });
            }
            NICKNAME => {
                card.nickname.push(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                });
            }
            PHOTO => match Uri::try_from(value.as_ref()) {
                Ok(uri) => {
                    let value = uri.into_owned();
                    card.photo.push(TextOrUriProperty::Uri(UriProperty {
                        value,
                        parameters,
                        group,
                    }));
                }
                Err(_) => {
                    card.photo.push(TextOrUriProperty::Text(TextProperty {
                        value: value.into_owned(),
                        parameters,
                        group,
                    }));
                }
            },
            BDAY => {
                if card.bday.is_some() {
                    return Err(Error::OnlyOnce(upper_name));
                }

                let prop = parse_date_time_or_text(
                    &upper_name,
                    value,
                    parameters,
                    group,
                )?;
                card.bday = Some(prop);
            }
            ANNIVERSARY => {
                if card.anniversary.is_some() {
                    return Err(Error::OnlyOnce(upper_name));
                }

                let prop = parse_date_time_or_text(
                    &upper_name,
                    value,
                    parameters,
                    group,
                )?;
                card.anniversary = Some(prop);
            }
            GENDER => {
                if card.gender.is_some() {
                    return Err(Error::OnlyOnce(upper_name));
                }
                let value: Gender = value.as_ref().parse()?;
                card.gender = Some(GenderProperty {
                    value,
                    parameters,
                    group,
                });
            }

            // Delivery Addressing
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.3
            ADR => {
                let value: DeliveryAddress = value.as_ref().parse()?;
                card.address.push(AddressProperty {
                    value,
                    parameters,
                    group,
                });
            }

            // Communications
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.4
            TEL => {
                let value = self.parse_text_or_uri(
                    value.as_ref(),
                    parameters,
                    group,
                )?;
                card.tel.push(value);
            }
            EMAIL => {
                card.email.push(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                });
            }
            IMPP => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.impp.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            LANG => {
                let value = parse_language_tag(value)?;
                card.lang.push(LanguageProperty {
                    value,
                    parameters,
                    group,
                });
            }

            // Geographic
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.5
            TZ => {
                let value_type = if let Some(parameters) = &parameters {
                    parameters.value.as_ref()
                } else {
                    None
                };

                if let Some(value_type) = value_type {
                    match value_type {
                        ValueType::UtcOffset => {
                            let mut value: UtcOffsetProperty =
                                value.as_ref().parse()?;
                            value.parameters = parameters;
                            value.group = group;
                            card.timezone
                                .push(TimeZoneProperty::UtcOffset(value));
                        }
                        ValueType::Uri => {
                            let value =
                                Uri::try_from(value.as_ref())?.into_owned();
                            card.timezone.push(TimeZoneProperty::Uri(
                                UriProperty {
                                    value,
                                    parameters,
                                    group,
                                },
                            ));
                        }
                        _ => {
                            return Err(Error::UnsupportedValueType(
                                value_type.to_string(),
                                upper_name,
                            ))
                        }
                    }
                } else {
                    card.timezone.push(TimeZoneProperty::Text(
                        TextProperty {
                            value: value.into_owned(),
                            parameters,
                            group,
                        },
                    ));
                }
            }
            GEO => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.geo.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }

            // Organizational
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.6
            TITLE => {
                card.title.push(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                });
            }
            ROLE => {
                card.role.push(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                });
            }
            LOGO => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.logo.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            ORG => {
                let value = value
                    .as_ref()
                    .split(';')
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                card.org.push(TextListProperty {
                    value,
                    parameters,
                    group,
                    delimiter: TextListDelimiter::SemiColon,
                });
            }
            MEMBER => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.member.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            RELATED => {
                let text_or_uri = self.parse_text_or_uri(
                    value.as_ref(),
                    parameters,
                    group,
                )?;
                card.related.push(text_or_uri);
            }

            // Explanatory
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.7
            CATEGORIES => {
                let value = value
                    .as_ref()
                    .split(',')
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                card.categories.push(TextListProperty {
                    value,
                    parameters,
                    group,
                    delimiter: TextListDelimiter::Comma,
                });
            }
            NOTE => {
                card.note.push(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                });
            }
            PRODID => {
                if card.prod_id.is_some() {
                    return Err(Error::OnlyOnce(upper_name));
                }
                card.prod_id = Some(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                });
            }
            REV => {
                if card.rev.is_some() {
                    return Err(Error::OnlyOnce(upper_name));
                }
                let value = parse_timestamp(value.as_ref())?;
                card.rev = Some(DateTimeProperty {
                    value,
                    parameters,
                    group,
                });
            }
            SOUND => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.sound.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            UID => {
                if card.uid.is_some() {
                    return Err(Error::OnlyOnce(upper_name));
                }
                let text_or_uri = self.parse_text_or_uri(
                    value.as_ref(),
                    parameters,
                    group,
                )?;
                card.uid = Some(text_or_uri);
            }
            CLIENTPIDMAP => {
                if let Some(params) = &parameters {
                    if params.pid.is_some() {
                        return Err(Error::ClientPidMapPidNotAllowed);
                    }
                }

                let value: ClientPidMap = value.as_ref().parse()?;
                card.client_pid_map.push(ClientPidMapProperty {
                    value,
                    parameters,
                    group,
                });
            }
            URL => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.url.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            VERSION => {
                return Err(Error::VersionMisplaced);
            }

            // Security
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.8
            KEY => {
                let text_or_uri = self.parse_text_or_uri(
                    value.as_ref(),
                    parameters,
                    group,
                )?;
                card.key.push(text_or_uri);
            }

            // Calendar
            // https://www.rfc-editor.org/rfc/rfc6350#section-6.9
            FBURL => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.fburl.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            CALADRURI => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.cal_adr_uri.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            CALURI => {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                card.cal_uri.push(UriProperty {
                    value,
                    parameters,
                    group,
                });
            }
            _ => return Err(Error::UnknownPropertyName(name.to_string())),
        }
        Ok(())
    }

    /// Parse a private extension property (`x-`) by name.
    fn parse_extension_property_by_name(
        &self,
        card: &mut Vcard,
        name: &str,
        value: Cow<'_, str>,
        parameters: Option<Parameters>,
        group: Option<String>,
    ) -> Result<()> {
        let value_type = if let Some(parameters) = &parameters {
            parameters.value.as_ref()
        } else {
            None
        };
        let prop = if let Some(value_type) = value_type {
            match value_type {
                ValueType::Text => AnyProperty::Text(value.into_owned()),
                ValueType::Integer => {
                    AnyProperty::Integer(parse_integer_list(value.as_ref())?)
                }
                ValueType::Float => {
                    AnyProperty::Float(parse_float_list(value.as_ref())?)
                }
                ValueType::Boolean => {
                    AnyProperty::Boolean(parse_boolean(value.as_ref())?)
                }
                ValueType::Date => {
                    AnyProperty::Date(parse_date_list(value.as_ref())?)
                }
                ValueType::DateTime => AnyProperty::DateTime(
                    parse_date_time_list(value.as_ref())?,
                ),
                ValueType::Time => {
                    AnyProperty::Time(parse_time_list(value.as_ref())?)
                }
                ValueType::DateAndOrTime => AnyProperty::DateAndOrTime(
                    parse_date_and_or_time_list(value.as_ref())?,
                ),
                ValueType::Timestamp => AnyProperty::Timestamp(
                    parse_timestamp_list(value.as_ref())?,
                ),
                ValueType::LanguageTag => {
                    AnyProperty::Language(parse_language_tag(value)?)
                }
                ValueType::UtcOffset => {
                    let value = parse_utc_offset(value.as_ref())?;
                    AnyProperty::UtcOffset(value)
                }
                ValueType::Uri => {
                    let value = Uri::try_from(value.as_ref())?.into_owned();
                    AnyProperty::Uri(value)
                }
            }
        } else {
            AnyProperty::Text(value.into_owned())
        };

        card.extensions.push(ExtensionProperty {
            name: name.to_string(),
            value: prop,
            group,
            parameters,
        });

        Ok(())
    }

    /// Get the slice for the property value.
    fn parse_property_value<'a>(
        &self,
        lex: &'a mut Lexer<'_, Token>,
    ) -> Result<Cow<'a, str>> {
        let mut first_range: Option<Range<usize>> = None;
        let mut last_range: Option<Range<usize>> = None;

        let mut needs_transform = false;
        let mut tokens = Vec::new();

        while let Some(token) = lex.next() {
            let span = lex.span();
            if first_range.is_none() {
                first_range = Some(span.clone());
            }

            if token == Ok(Token::Control) {
                return Err(Error::ControlCharacter(escape_control(
                    lex.slice(),
                )));
            }

            if token == Ok(Token::FoldedLine)
                || token == Ok(Token::EscapedSemiColon)
                || token == Ok(Token::EscapedComma)
                || token == Ok(Token::EscapedNewLine)
                || token == Ok(Token::EscapedBackSlash)
            {
                needs_transform = true;
            }

            if token == Ok(Token::NewLine) {
                last_range = Some(span);
                break;
            }

            tokens.push((token, span));
        }

        if let (Some(first), Some(last)) = (first_range, last_range) {
            if needs_transform {
                let mut value = String::new();
                for (token, span) in tokens {
                    if token == Ok(Token::FoldedLine) {
                        continue;
                    } else if token == Ok(Token::EscapedComma) {
                        value.push(',');
                        continue;
                    } else if token == Ok(Token::EscapedSemiColon) {
                        value.push(';');
                        continue;
                    } else if token == Ok(Token::EscapedNewLine) {
                        value.push('\n');
                        continue;
                    } else if token == Ok(Token::EscapedBackSlash) {
                        value.push('\\');
                        continue;
                    }

                    let source = lex.source();
                    value.push_str(&source[span]);
                }
                Ok(Cow::Owned(value))
            } else {
                let source = lex.source();
                Ok(Cow::Borrowed(&source[first.start..last.start]))
            }
        } else {
            Err(Error::InvalidPropertyValue)
        }
    }

    /// Parse text or Uri from a value.
    fn parse_text_or_uri<S: AsRef<str>>(
        &self,
        value: S,
        parameters: Option<Parameters>,
        group: Option<String>,
    ) -> Result<TextOrUriProperty> {
        let value_type = if let Some(parameters) = &parameters {
            parameters.value.as_ref()
        } else {
            None
        };
        if let Some(value_type) = value_type {
            if let ValueType::Text = value_type {
                Ok(TextOrUriProperty::Text(TextProperty {
                    value: value.as_ref().to_string(),
                    parameters,
                    group,
                }))
            } else if let ValueType::Uri = value_type {
                let value = Uri::try_from(value.as_ref())?.into_owned();
                Ok(TextOrUriProperty::Uri(UriProperty {
                    value,
                    parameters,
                    group,
                }))
            } else {
                Err(Error::UnknownValueType(value_type.to_string()))
            }
        } else {
            match Uri::try_from(value.as_ref()) {
                Ok(value) => Ok(TextOrUriProperty::Uri(UriProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                })),
                Err(_) => Ok(TextOrUriProperty::Text(TextProperty {
                    value: value.as_ref().to_string(),
                    parameters,
                    group,
                })),
            }
        }
    }

    /// Assert we have an expected token.
    fn assert_token(
        &self,
        value: Option<&LexResult<Token>>,
        expected: &[Token],
    ) -> Result<()> {
        if let Some(Ok(value)) = value {
            if expected.contains(value) {
                Ok(())
            } else {
                Err(Error::IncorrectToken(format!("{:#?}", value)))
            }
        } else {
            Err(Error::TokenExpected)
        }
    }
}

fn parse_date_time_or_text(
    prop_name: &str,
    value: Cow<'_, str>,
    parameters: Option<Parameters>,
    group: Option<String>,
) -> Result<DateTimeOrTextProperty> {
    let value_type = if let Some(parameters) = &parameters {
        parameters.value.as_ref()
    } else {
        None
    };

    if let Some(value_type) = value_type {
        match value_type {
            ValueType::Text => {
                Ok(DateTimeOrTextProperty::Text(TextProperty {
                    value: value.into_owned(),
                    parameters,
                    group,
                }))
            }
            ValueType::DateAndOrTime => {
                let value = parse_date_and_or_time_list(value.as_ref())?;
                Ok(DateTimeOrTextProperty::DateTime(DateAndOrTimeProperty {
                    value,
                    parameters,
                    group,
                }))
            }
            _ => Err(Error::UnsupportedValueType(
                value_type.to_string(),
                String::from(prop_name),
            )),
        }
    } else {
        let value = parse_date_and_or_time_list(value.as_ref())?;
        Ok(DateTimeOrTextProperty::DateTime(DateAndOrTimeProperty {
            value,
            parameters,
            group,
        }))
    }
}

#[cfg(feature = "mime")]
fn parse_media_type(value: String, params: &mut Parameters) -> Result<()> {
    let mime: Mime = value.parse()?;
    params.media_type = Some(mime);
    Ok(())
}

#[cfg(not(feature = "mime"))]
fn parse_media_type(value: String, params: &mut Parameters) -> Result<()> {
    params.media_type = Some(value);
    Ok(())
}

#[cfg(feature = "language-tags")]
fn parse_language_tag(value: Cow<'_, str>) -> Result<LanguageTag> {
    let tag: LanguageTag = value.as_ref().parse()?;
    Ok(tag)
}

#[cfg(not(feature = "language-tags"))]
fn parse_language_tag(value: Cow<'_, str>) -> Result<String> {
    Ok(value.into_owned())
}
