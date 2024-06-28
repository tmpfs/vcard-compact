# vCard

Fast and correct vCard parser based on [RFC6350](https://www.rfc-editor.org/rfc/rfc6350); see the [API documentation](https://docs.rs/vcard4/latest/vcard4/) for more information.

For interoperability with older software the parser will accept input with a `CHARSET` parameter that has a value of `UTF-8`, any other encoding value for `CHARSET` will generate an error. However, this parameter is not part of [RFC6350](https://www.rfc-editor.org/rfc/rfc6350) and is therefore not included in the string output for a vCard.

License is MIT or Apache-2.0.
