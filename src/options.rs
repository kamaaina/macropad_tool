use std::num::ParseIntError;

use crate::consts::VENDOR_ID;
use crate::parse;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    pub command: Command,

    #[clap(flatten)]
    pub devel_options: DevelOptions,
}

#[derive(Args)]
#[clap(hide(true), next_help_heading = "Internal options (use with caution)")]
pub struct DevelOptions {
    #[arg(long, default_value_t=VENDOR_ID, value_parser=hex_or_decimal, hide=true)]
    pub vendor_id: u16,

    #[arg(long, value_parser=hex_or_decimal, hide=true)]
    pub product_id: Option<u16>,

    #[arg(long, value_parser=parse_address, hide=true)]
    pub address: Option<(u8, u8)>,

    #[arg(long, default_value_t = 0x4, hide = true)]
    pub endpoint_address: u8,

    #[arg(long, hide = true)]
    pub interface_number: Option<u8>,
}

pub fn hex_or_decimal(s: &str) -> Result<u16, ParseIntError> {
    if s.to_ascii_lowercase().starts_with("0x") {
        u16::from_str_radix(&s[2..], 16)
    } else {
        u16::from_str_radix(s, 10)
    }
}

fn parse_address(s: &str) -> std::result::Result<(u8, u8), nom::error::Error<String>> {
    parse::from_str(parse::address, s)
}

#[derive(Subcommand)]
pub enum Command {
    /// Show supported keys and modifiers
    #[clap(hide = true)]
    ShowKeys,

    /// Validate key mappings config on stdin
    Validate,

    /// Program key mappings from stdin to device
    Program,

    /// Select LED backlight mode
    #[clap(hide = true)]
    Led(LedCommand),
}

#[derive(Parser)]
pub struct LedCommand {
    /// Index of LED mode (zero-based)
    pub index: u8,
}
