use anyhow::{anyhow, Result};

use crate::decoder::{DeviceInformation, KeyCode};

pub struct Printer {}

impl Printer {
    pub fn to_yaml(device: &DeviceInformation, keys: &Vec<KeyCode>) {
        let rows_cols = Self::key_breakdown(device.num_keys).expect("rows/cols mapping");
        println!("orientation: normal\n");
        println!("rows: {}", rows_cols.0);
        println!("colums: {}", rows_cols.1);
        println!("knobs: {}\n", device.num_encoders);
        println!("layers:\n");
        println!("  - buttons:");
        for key in keys {
            //
            println!("     - [");
        }
    }

    fn key_breakdown(num_keys: u8) -> Result<(u8 /*rows*/, u8 /*cols*/)> {
        match num_keys {
            3 => Ok((1, 3)),
            6 => Ok((2, 3)),
            9 => Ok((3, 3)),
            12 => Ok((3, 4)),
            _ => Err(anyhow!(
                "unable to guestimate the row/cols for this macropad"
            )),
        }
    }
}
