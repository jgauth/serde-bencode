use std::fmt::{self, Display};

use serde::de;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum Error {
    Message(String),

    // integer
    NegativeZero,
    NonASCII,
    ExpectedInteger,
    ExpectedI,
    ExpectedE,

    // bytes
    ZeroLength,
    NegativeLength,
    ExpectedColon,

    // dictionary
    NonLexicographical,
    ExpectedDict,
    ExpectedDictEnd,

    // list
    ExpectedList,
    ExpectedListEnd,

    TrailingCharacters,
    Eof,
    Syntax,
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Message(msg.to_string())
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Message(msg) => f.write_str(msg),
            Error::NegativeZero => f.write_str("disallowed negative zero"),
            Error::NonASCII => f.write_str("disallowed non-ascii character"),
            Error::ExpectedInteger => f.write_str("expected an integer"),
            Error::ZeroLength => f.write_str("disallowed zero-length byte string"),
            Error::NegativeLength => f.write_str("disallowed negative length bytes string"),
            Error::NonLexicographical => f.write_str("keys not lexicographically sorted"),
            Error::TrailingCharacters => f.write_str("unexpected trailing characters"),
            Error::Eof => f.write_str("End of fuck"),
            Error::ExpectedColon => f.write_str("Expected a colon between length and string"),
            _ => f.write_str("shit"),
        }
    }
}
