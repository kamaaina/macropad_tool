//! Collection of NOM parsers for various things.
//! Generally only `parse` and `from_str` functions should be called
//! from outside of this module, they ensures that whole input is
//! consumed.
//! Other functions are composable parsers for use within this module
//! or as parameters for functions mentioned above.

use nom::{
    character::complete::{char, digit1},
    combinator::{all_consuming, map_res},
    error::ParseError,
    sequence::separated_pair,
    IResult, InputLength, Parser,
};

use std::str::FromStr;

pub fn address(s: &str) -> IResult<&str, (u8, u8)> {
    let byte = || map_res(digit1, u8::from_str);
    let mut parser = separated_pair(byte(), char(':'), byte());
    parser(s)
}

/// Parses string with given parser ensuring that whole input is consumed.
pub fn parse<I, O, E, P>(parser: P, input: I) -> std::result::Result<O, E>
where
    I: InputLength,
    E: ParseError<I>,
    P: Parser<I, O, E>,
{
    use nom::Finish as _;
    all_consuming(parser)(input)
        .finish()
        .map(|(_, value)| value)
}

/// Parses string using given parser, as `parse` do, but also converts string reference
/// in returned error to String, so it may be used in implementations of `FromStr`.
pub fn from_str<O, P>(parser: P, s: &str) -> std::result::Result<O, nom::error::Error<String>>
where
    for<'a> P: Parser<&'a str, O, nom::error::Error<&'a str>>,
{
    match parse(parser, s) {
        Ok(value) => Ok(value),
        Err(nom::error::Error { input, code }) => Err(nom::error::Error {
            input: input.to_owned(),
            code,
        }),
    }
}
