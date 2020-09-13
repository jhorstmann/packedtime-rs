#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidLen(usize),
    InvalidChar(usize),
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;