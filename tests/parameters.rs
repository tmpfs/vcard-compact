mod test_helpers;

use anyhow::Result;
use uriparse::uri::URI as Uri;

use vcard4::{
    helper::parse_utc_offset,
    parameter::{
        Pid, RelatedType, TelephoneType, TimeZoneParameter, TypeParameter,
        ValueType,
    },
    parse, Error,
};

use test_helpers::{assert_language, assert_media_type, assert_round_trip};

#[test]
fn parse_parameters() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN:Mr. John Q. Public\, Esq.
NICKNAME;LANGUAGE=en;TYPE=work:Boss
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());

    let card = vcards.remove(0);

    let fname = card.formatted_name.get(0).unwrap();
    assert_eq!("Mr. John Q. Public, Esq.", fname.value);

    let nickname = card.nickname.get(0).unwrap();
    assert_eq!("Boss", nickname.value);
    assert!(nickname.parameters.is_some());

    //let tag: LanguageTag = "en".parse()?;
    let parameters = nickname.parameters.as_ref().unwrap();

    let param: TypeParameter = "work".parse()?;
    //assert_eq!(Some(tag), parameters.language);
    assert_eq!(&vec![param], parameters.types.as_ref().unwrap());
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_language() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN:Jane Doe
ROLE;LANGUAGE=tr:hoca
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.role.get(0).unwrap();
    assert_language(
        prop.parameters.as_ref().unwrap().language.as_ref().unwrap(),
        "tr",
    )?;
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_value() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;VALUE=text:Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        &ValueType::Text,
        prop.parameters.as_ref().unwrap().value.as_ref().unwrap()
    );
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_pref() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;PREF=1:Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(1, prop.parameters.as_ref().unwrap().pref.unwrap());
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_altid() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;ALTID=1:Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        "1",
        prop.parameters.as_ref().unwrap().alt_id.as_ref().unwrap()
    );
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_pid() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;PID=1:Jane Doe
FN;PID=1.1:Jane Doe Smith
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        &vec![Pid::new(1, None)],
        prop.parameters.as_ref().unwrap().pid.as_ref().unwrap()
    );
    let prop = card.formatted_name.get(1).unwrap();
    assert_eq!(
        &vec![Pid::new(1, Some(1))],
        prop.parameters.as_ref().unwrap().pid.as_ref().unwrap()
    );
    assert_round_trip(&card)?;
    Ok(())
}

fn assert_param_type(value: TypeParameter) -> Result<()> {
    let input = format!(
        r#"BEGIN:VCARD
VERSION:4.0
FN;TYPE={}:Jane Doe
END:VCARD"#,
        value
    );

    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        &vec![value],
        prop.parameters.as_ref().unwrap().types.as_ref().unwrap()
    );
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_type() -> Result<()> {
    assert_param_type(TypeParameter::Home)?;
    assert_param_type(TypeParameter::Work)?;

    assert_param_type(TypeParameter::Telephone(TelephoneType::Text))?;
    assert_param_type(TypeParameter::Telephone(TelephoneType::Voice))?;
    assert_param_type(TypeParameter::Telephone(TelephoneType::Fax))?;
    assert_param_type(TypeParameter::Telephone(TelephoneType::Cell))?;
    assert_param_type(TypeParameter::Telephone(TelephoneType::Video))?;
    assert_param_type(TypeParameter::Telephone(TelephoneType::Pager))?;
    assert_param_type(TypeParameter::Telephone(TelephoneType::TextPhone))?;

    assert_param_type(TypeParameter::Related(RelatedType::Contact))?;

    assert_param_type(TypeParameter::Related(RelatedType::Acquaintance))?;
    assert_param_type(TypeParameter::Related(RelatedType::Friend))?;
    assert_param_type(TypeParameter::Related(RelatedType::Met))?;
    assert_param_type(TypeParameter::Related(RelatedType::CoWorker))?;
    assert_param_type(TypeParameter::Related(RelatedType::Colleague))?;
    assert_param_type(TypeParameter::Related(RelatedType::CoResident))?;
    assert_param_type(TypeParameter::Related(RelatedType::Neighbor))?;
    assert_param_type(TypeParameter::Related(RelatedType::Child))?;
    assert_param_type(TypeParameter::Related(RelatedType::Parent))?;
    assert_param_type(TypeParameter::Related(RelatedType::Sibling))?;
    assert_param_type(TypeParameter::Related(RelatedType::Spouse))?;
    assert_param_type(TypeParameter::Related(RelatedType::Kin))?;
    assert_param_type(TypeParameter::Related(RelatedType::Muse))?;
    assert_param_type(TypeParameter::Related(RelatedType::Crush))?;
    assert_param_type(TypeParameter::Related(RelatedType::Date))?;
    assert_param_type(TypeParameter::Related(RelatedType::Sweetheart))?;
    assert_param_type(TypeParameter::Related(RelatedType::Me))?;
    assert_param_type(TypeParameter::Related(RelatedType::Agent))?;
    assert_param_type(TypeParameter::Related(RelatedType::Emergency))?;

    assert_param_type(TypeParameter::Extension("foo".to_string()))?;

    Ok(())
}

#[test]
fn param_mediatype() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;MEDIATYPE=text/plain;VALUE=text:Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_media_type(prop.parameters.as_ref(), "text/plain")?;
    assert_eq!(
        &ValueType::Text,
        prop.parameters.as_ref().unwrap().value.as_ref().unwrap()
    );
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_calscale() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;CALSCALE=gregorian:Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        "gregorian",
        prop.parameters.as_ref().unwrap().calscale.as_ref().unwrap()
    );
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_sort_as() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;SORT-AS="Doe,Jane":Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        &vec!["Doe", "Jane"],
        prop.parameters.as_ref().unwrap().sort_as.as_ref().unwrap()
    );
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_geo() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;GEO="geo:37.386013\,-122.082932":Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        "geo:37.386013,-122.082932",
        &prop
            .parameters
            .as_ref()
            .unwrap()
            .geo
            .as_ref()
            .unwrap()
            .to_string()
    );
    assert_round_trip(&card)?;
    Ok(())
}

#[test]
fn param_tz() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;TZ=-0500:Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        &TimeZoneParameter::UtcOffset(parse_utc_offset("-0500")?),
        prop.parameters.as_ref().unwrap().timezone.as_ref().unwrap()
    );
    assert_round_trip(&card)?;

    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;TZ=Raleigh/North America:Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        &TimeZoneParameter::Text(String::from("Raleigh/North America")),
        prop.parameters.as_ref().unwrap().timezone.as_ref().unwrap()
    );
    assert_round_trip(&card)?;

    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;TZ="https://example.com/tz-database/acdt":Jane Doe
END:VCARD"#;
    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!(
        &TimeZoneParameter::Uri(
            Uri::try_from("https://example.com/tz-database/acdt")?
                .into_owned()
        ),
        prop.parameters.as_ref().unwrap().timezone.as_ref().unwrap()
    );
    assert_round_trip(&card)?;

    Ok(())
}

#[test]
fn param_charset() -> Result<()> {
    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;CHARSET=UTF-8:Jane Doe
ORG:Some Organization
TITLE;CHARSET=UTF-8:External employee
KIND:individual
N;CHARSET=UTF-8:Doe;Jane;;;
END:VCARD"#;

    let expected = r#"BEGIN:VCARD
VERSION:4.0
KIND:individual
FN:Jane Doe
N:Doe;Jane;;;
TITLE:External employee
ORG:Some Organization
END:VCARD
"#
    .replace('\n', "\r\n");

    let mut vcards = parse(input)?;
    assert_eq!(1, vcards.len());
    let card = vcards.remove(0);
    let prop = card.formatted_name.get(0).unwrap();
    assert_eq!("Jane Doe", prop.value,);
    assert_eq!(expected, card.to_string());

    let input = r#"BEGIN:VCARD
VERSION:4.0
FN;CHARSET=ISO-8859-1:Jane Doe
N;CHARSET=UTF-8:Doe;Jane;;;
END:VCARD"#;
    let err = parse(input).expect_err("Non-UTF-8 CHARSET should fail");
    assert!(
        matches!(&err, Error::CharsetParameter(x) if x == "ISO-8859-1"),
        "Unexpected error: {err:?}"
    );

    Ok(())
}
