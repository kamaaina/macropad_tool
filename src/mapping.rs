use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Macropad {
    pub device: Device,
    pub layers: Vec<Layer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub orientation: String,
    pub rows: u8,
    pub cols: u8,
    pub knobs: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Layer {
    pub buttons: Vec<Vec<String>>,
    pub knobs: Vec<Knob>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Knob {
    pub ccw: String,
    pub click: String,
    pub cw: String,
}

use ron::de::from_reader;
use ron::ser::{to_string_pretty, PrettyConfig};
use std::fs::File;

fn read() -> Macropad {
    // read configuration
    let cfg_file = "./mapping.ron";
    println!("configuration file: {}", cfg_file);
    let f = File::open(cfg_file).expect("Failed opening file");
    let config: Macropad = match from_reader(f) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };
    config
}

fn print(config: Macropad) {
    let pretty = PrettyConfig::new()
        .depth_limit(4)
        .separate_tuple_members(true)
        .enumerate_arrays(false);

    let s = to_string_pretty(&config, pretty).expect("Serialization failed");
    println!("------------------------------");
    println!("{s}");
}

fn validate(config: Macropad) -> anyhow::Result<()> {
    Err(anyhow!("failed because"))
}

#[cfg(test)]
mod tests {

    use crate::mapping::{print, read, validate};

    #[test]
    fn mapping_read() {
        read();
    }

    #[test]
    fn mapping_print() {
        print(read());
    }

    #[test]
    fn mapping_validate() -> anyhow::Result<()> {
        validate(read())?;
        Ok(())
    }
}
