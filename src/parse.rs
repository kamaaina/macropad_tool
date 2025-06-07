//! Collection of NOM parsers for various things.
//! Generally only `parse` and `from_str` functions should be called
//! from outside of this module, they ensures that whole input is
//! consumed.
//! Other functions are composable parsers for use within this module
//! or as parameters for functions mentioned above.

use nom::{
    character::complete::{char, digit1},
    combinator::{all_consuming, map_res},
    error::{Error as NomError, ParseError},
    sequence::separated_pair,
    Finish, IResult, Parser,
};
use std::str::FromStr;

/// Parses a string like "12:34" into (u8, u8)
pub fn address(input: &str) -> IResult<&str, (u8, u8)> {
    let byte = map_res(digit1, u8::from_str);
    separated_pair(byte, char(':'), map_res(digit1, u8::from_str)).parse(input)
}

/// Runs a parser and ensures the entire input is consumed
pub fn parse<'a, O, E, P>(parser: P, input: &'a str) -> Result<O, E>
where
    P: Parser<&'a str, Output = O, Error = E>,
    E: ParseError<&'a str>,
{
    all_consuming(parser)
        .parse(input)
        .finish()
        .map(|(_, out)| out)
}

/// Like `parse`, but converts error input to `String` for `FromStr`
pub fn from_str<O, P>(parser: P, s: &str) -> Result<O, NomError<String>>
where
    for<'a> P: Parser<&'a str, Output = O, Error = NomError<&'a str>>,
{
    match parse(parser, s) {
        Ok(value) => Ok(value),
        Err(NomError { input, code }) => Err(NomError {
            input: input.to_string(),
            code,
        }),
    }
}
