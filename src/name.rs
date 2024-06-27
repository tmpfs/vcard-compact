/// Constants for property and parameter names.

pub(crate) const HOME: &str = "home";
pub(crate) const WORK: &str = "work";

pub(crate) const BEGIN: &str = "BEGIN:VCARD";
pub(crate) const VERSION_4: &str = "VERSION:4.0";
pub(crate) const END: &str = "END:VCARD";

// Property
pub(crate) const VERSION: &str = "VERSION";
pub(crate) const SOURCE: &str = "SOURCE";
pub(crate) const KIND: &str = "KIND";
pub(crate) const FN: &str = "FN";
pub(crate) const N: &str = "N";
pub(crate) const NICKNAME: &str = "NICKNAME";
pub(crate) const PHOTO: &str = "PHOTO";
pub(crate) const BDAY: &str = "BDAY";
pub(crate) const ANNIVERSARY: &str = "ANNIVERSARY";
pub(crate) const GENDER: &str = "GENDER";
pub(crate) const ADR: &str = "ADR";
pub(crate) const TEL: &str = "TEL";
pub(crate) const EMAIL: &str = "EMAIL";
pub(crate) const IMPP: &str = "IMPP";
pub(crate) const LANG: &str = "LANG";
pub(crate) const TZ: &str = "TZ";
pub(crate) const GEO: &str = "GEO";
pub(crate) const TITLE: &str = "TITLE";
pub(crate) const ROLE: &str = "ROLE";
pub(crate) const LOGO: &str = "LOGO";
pub(crate) const ORG: &str = "ORG";
pub(crate) const MEMBER: &str = "MEMBER";
pub(crate) const RELATED: &str = "RELATED";
pub(crate) const CATEGORIES: &str = "CATEGORIES";
pub(crate) const NOTE: &str = "NOTE";
pub(crate) const PRODID: &str = "PRODID";
pub(crate) const REV: &str = "REV";
pub(crate) const SOUND: &str = "SOUND";
pub(crate) const UID: &str = "UID";
pub(crate) const CLIENTPIDMAP: &str = "CLIENTPIDMAP";
pub(crate) const URL: &str = "URL";
pub(crate) const KEY: &str = "KEY";
pub(crate) const FBURL: &str = "FBURL";
pub(crate) const CALADRURI: &str = "CALADRURI";
pub(crate) const CALURI: &str = "CALURI";
pub(crate) const XML: &str = "XML";

// Parameter
pub(crate) const LANGUAGE: &str = "LANGUAGE";
pub(crate) const VALUE: &str = "VALUE";
pub(crate) const PREF: &str = "PREF";
pub(crate) const ALTID: &str = "ALTID";
pub(crate) const PID: &str = "PID";
pub(crate) const TYPE: &str = "TYPE";
pub(crate) const MEDIATYPE: &str = "MEDIATYPE";
pub(crate) const CALSCALE: &str = "CALSCALE";
pub(crate) const SORT_AS: &str = "SORT-AS";
// NOTE: we use GEO from the property names
// NOTE: we use TZ from the property names
pub(crate) const LABEL: &str = "LABEL";
// RFC 6350 removed the CHARSET parameter because it requires UTF-8, but some
// implementations still emit CHARSET=UTF-8. This is the only value we allow.
pub(crate) const CHARSET: &str = "CHARSET";

// Apple uses this for embedded photos
pub(crate) const ENCODING: &str = "ENCODING";
