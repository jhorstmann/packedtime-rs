#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidLen(usize),
    InvalidChar(usize),
    TrailingChar(usize),
    InvalidValue()
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;