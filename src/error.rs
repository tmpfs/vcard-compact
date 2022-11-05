use thiserror::Error;

/// Errors generated by the vCard library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("input token was expected but reached EOF")]
    TokenExpected,

    #[error("input token was incorrect")]
    IncorrectToken,

    #[error("property name '{0}' is not supported")]
    UnknownPropertyName(String),

    #[error("property value is invalid")]
    InvalidPropertyValue,

    #[error("property or parameter delimiter expected")]
    DelimiterExpected,

    #[error("parameter name '{0}' is not supported")]
    UnknownParameterName(String),

    #[error("value type '{0}' is not supported")]
    UnknownValueType(String),

    #[error("related type value '{0}' is not supported")]
    UnknownRelatedTypeValue(String),

    #[error("value type '{0}' is not supported in this context '{1}'")]
    UnsupportedValueType(String, String),

    #[error("kind '{0}' is not supported")]
    UnknownKind(String),

    #[error("sex '{0}' is not supported")]
    UnknownSex(String),

    #[error("gender value is missing sex")]
    NoSex,

    #[error("property '{0}' may only appear exactly once")]
    OnlyOnce(String),

    #[error("formatted name (FN) is required")]
    NoFormattedName,

    #[error("value '{0}' for UTC offset is invalid")]
    InvalidUtcOffset(String),

    #[error(transparent)]
    LanguageParse(#[from] language_tags::ParseError),

    #[error(transparent)]
    UriParse(#[from] fluent_uri::ParseError),

    #[error(transparent)]
    ComponentRange(#[from] time::error::ComponentRange),

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}
