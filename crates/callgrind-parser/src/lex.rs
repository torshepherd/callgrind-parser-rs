//! Stateless token grammar. Complete nom parsers operate on a framed line;
//! incomplete I/O chunks are handled by BufRead, not mistaken for syntax errors.
use crate::ParseErrorKind;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit1, hex_digit1},
    combinator::{all_consuming, recognize},
    sequence::preceded,
};

fn unsigned_token(input: &str) -> IResult<&str, &str> {
    alt((recognize(preceded(tag("0x"), hex_digit1)), digit1)).parse(input)
}

pub(crate) fn number(input: &str) -> Result<u64, ParseErrorKind> {
    all_consuming(unsigned_token)
        .parse(input)
        .map_err(|_| ParseErrorKind::InvalidNumber)?;
    let result = if let Some(hex) = input.strip_prefix("0x") {
        u64::from_str_radix(hex, 16)
    } else {
        input.parse()
    };
    result.map_err(|_| ParseErrorKind::NumberOverflow)
}

pub(crate) fn position(input: &str, previous: u64) -> Result<u64, ParseErrorKind> {
    if input == "*" {
        return Ok(previous);
    }
    if let Some(value) = input.strip_prefix('+') {
        return previous
            .checked_add(number(value)?)
            .ok_or(ParseErrorKind::NumberOverflow);
    }
    if let Some(value) = input.strip_prefix('-') {
        return previous
            .checked_sub(number(value)?)
            .ok_or(ParseErrorKind::PositionUnderflow);
    }
    number(input)
}

pub(crate) fn expression(input: &str) -> Result<Vec<(u64, &str)>, ParseErrorKind> {
    input
        .split('+')
        .map(|term| {
            let term = term.trim();
            let (coefficient, name) = if term.starts_with(|c: char| c.is_ascii_digit()) {
                let (rest, digits) =
                    unsigned_token(term).map_err(|_| ParseErrorKind::InvalidEventDefinition)?;
                (
                    number(digits)?,
                    rest.trim_start()
                        .strip_prefix('*')
                        .unwrap_or(rest.trim_start())
                        .trim(),
                )
            } else {
                (1, term)
            };
            if !name.starts_with(|c: char| c.is_ascii_alphabetic())
                || !name.chars().all(|c| c.is_ascii_alphanumeric())
            {
                return Err(ParseErrorKind::InvalidEventDefinition);
            }
            Ok((coefficient, name))
        })
        .collect()
}
