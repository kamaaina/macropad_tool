use anyhow::{anyhow, Result};
use log::debug;
use serde::{Deserialize, Serialize};

/// Mapping configuration of a macropad
#[derive(Debug, Serialize, Deserialize)]
pub struct Macropad {
    /// Device configuration
    pub device: Device,
    /// Layer configuration
    pub layers: Vec<Layer>,
}

impl Macropad {
    /// Creates a new Macropad with the specified rows, cols, and knobs
    ///
    /// #Arguments
    /// `rows` - number of rows
    /// `cols` - number of columns
    /// `knobs` - number of rotary encoders
    ///
    pub fn new(rows: u8, cols: u8, knobs: u8) -> Self {
        Self {
            device: Device {
                orientation: Orientation::Normal,
                rows,
                cols,
                knobs,
            },
            layers: vec![
                Layer::new(rows, cols, knobs),
                Layer::new(rows, cols, knobs),
                Layer::new(rows, cols, knobs),
            ],
        }
    }
}

/// Device configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    /// Orientation of device
    pub orientation: Orientation,
    /// Number of rows
    pub rows: u8,
    /// Number of columns
    pub cols: u8,
    /// Number of knobs
    pub knobs: u8,
}

/// Layer configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct Layer {
    /// Key mappings
    pub buttons: Vec<Vec<Button>>,
    /// Rotary encoder mappings
    pub knobs: Vec<Knob>,
}

impl Layer {
    /// Creates a new empty mapping structure for a layer given device configuration
    ///
    /// #Arguments
    /// `rows` - number of rows
    /// `cols` - number of columns
    /// `knobs` - number of rotary encoders
    ///
    pub fn new(rows: u8, cols: u8, num_knobs: u8) -> Self {
        let mut buttons = Vec::new();
        for _i in 0..rows {
            buttons.push(vec![Button::new(); cols.into()]);
        }

        let mut knobs = Vec::new();
        for _i in 0..num_knobs {
            knobs.push(Knob {
                ccw: Button::new(),
                press: Button::new(),
                cw: Button::new(),
            });
        }
        Self { buttons, knobs }
    }
}

/// Mapping for a button
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Button {
    /// Delay value (only used if mapping is a keychord; has ',' for key presses)
    pub delay: u16,
    /// Mapping for the button
    pub mapping: String,
}

impl Button {
    /// Creates a new Button with 0 delay and empty mapping
    ///
    pub fn new() -> Self {
        Self {
            delay: 0,
            mapping: String::new(),
        }
    }
}

/// Mapping for a knob
#[derive(Debug, Serialize, Deserialize)]
pub struct Knob {
    /// Counter-Clockwise turn
    pub ccw: Button,
    /// Pressing the knob
    pub press: Button,
    /// Clockwise turn
    pub cw: Button,
}

use ron::de::from_reader;
use ron::ser::{to_string_pretty, PrettyConfig};
use std::fs::File;
use std::str::FromStr;

use crate::config::Orientation;
use crate::consts;
use crate::keyboard::{MediaCode, Modifier, WellKnownCode};

pub struct Mapping {}

impl Mapping {
    /// Reads the specified configuration file and returns a Macropad
    ///
    /// #Arguments
    /// `cfg_file` - configuration file to be read and parsed
    ///
    pub fn read(cfg_file: &str) -> Macropad {
        debug!("configuration file: {cfg_file}");
        let f = File::open(cfg_file).expect("Failed opening file");
        let config: Macropad = match from_reader(f) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {e}");
                std::process::exit(1);
            }
        };
        config
    }

    /// Prints the Macropad to stdout
    ///
    /// #Arguments
    /// `config` - macropad to be printed
    ///
    pub fn print(config: Macropad) {
        let pretty = PrettyConfig::new()
            .depth_limit(4)
            .separate_tuple_members(true)
            .enumerate_arrays(false);

        let s = to_string_pretty(&config, pretty).expect("Serialization failed");
        println!("{s}");
    }

    /// Validates the configuration against the specified product ID. If the product ID
    ///  is not specified, does general validation. Returns `Result<Ok()>` on success; Err
    /// otherwise
    ///
    /// #Arguments
    /// `cfg_file` - configuration file to validate
    /// `pid` - Optional product id to validate against
    ///
    pub fn validate(cfg_file: &str, pid: Option<u16>) -> anyhow::Result<()> {
        // get the maximum number a key can be programmed for
        let mut max_programmable_keys = 0xff;
        if let Some(max) = pid {
            match max {
                0x8840 | 0x8842 => max_programmable_keys = consts::MAX_KEY_PRESSES_884X,
                0x8890 => max_programmable_keys = consts::MAX_KEY_PRESSES_8890,
                _ => {
                    let err_msg = format!("Unknown product id 0x{:02x}", pid.unwrap());
                    return Err(anyhow!(err_msg));
                }
            }
        }
        debug!("max_programmable_keys: {max_programmable_keys}");
        debug!("pid: {pid:?}");

        // check layers
        let cfg = Self::read(cfg_file);

        if cfg.layers.is_empty() || cfg.layers.len() > 3 {
            return Err(anyhow!("number of layers must be > 0 and < 4"));
        }

        // check rows/cols/knobs
        for (i, layer) in cfg.layers.iter().enumerate() {
            // row check
            if layer.buttons.len() != cfg.device.rows.into() {
                return Err(anyhow!(
                    "number of rows mismatch at layer {}. Expected {} rows found {}",
                    i + 1,
                    cfg.device.rows,
                    layer.buttons.len(),
                ));
            }

            // column check
            for (j, btn_mapping) in layer.buttons.iter().enumerate() {
                if btn_mapping.len() != cfg.device.cols.into() {
                    return Err(anyhow!(
                        "number of colums mismatch at layer {} row {}. Expected {} columns found {}",
                        i + 1,
                        j + 1,
                        cfg.device.cols,
                        btn_mapping.len()
                    ));
                }

                // check the individual button
                for (k, btn) in btn_mapping.iter().enumerate() {
                    let retval = Self::validate_key_mapping(btn, max_programmable_keys, pid);
                    if retval.is_err() {
                        return Err(anyhow!(
                            "{} -- '{}' at layer {} row {} button {}",
                            retval.err().unwrap(),
                            btn.mapping,
                            i + 1,
                            j + 1,
                            k + 1
                        ));
                    }
                }
            }

            // knob check
            if layer.knobs.len() != cfg.device.knobs.into() {
                return Err(anyhow!(
                    "number of knobs mismatch at layer {}. Expected {} knobs found {}",
                    i + 1,
                    cfg.device.knobs,
                    layer.knobs.len(),
                ));
            }

            // knob button mapping
            for (k, knob) in layer.knobs.iter().enumerate() {
                let retval = Self::validate_key_mapping(&knob.ccw, max_programmable_keys, pid);
                if retval.is_err() {
                    return Err(anyhow!(
                        "{} - key '{}' at layer {} knob {} in ccw",
                        retval.err().unwrap(),
                        &knob.ccw.mapping,
                        i + 1,
                        k + 1
                    ));
                }
                let retval = Self::validate_key_mapping(&knob.press, max_programmable_keys, pid);
                if retval.is_err() {
                    return Err(anyhow!(
                        "{} - key '{}' at layer {} knob {} in press",
                        retval.err().unwrap(),
                        &knob.press.mapping,
                        i + 1,
                        k + 1
                    ));
                }
                let retval = Self::validate_key_mapping(&knob.cw, max_programmable_keys, pid);
                if retval.is_err() {
                    return Err(anyhow!(
                        "{} - key '{}' at layer {} knob {} in cw",
                        retval.err().unwrap(),
                        &knob.cw.mapping,
                        i + 1,
                        k + 1
                    ));
                }
            }
        }

        Ok(())
    }

    fn validate_key_mapping(btn: &Button, max_size: usize, pid: Option<u16>) -> Result<()> {
        // ensure we don't go over max
        let keys: Vec<_> = btn.mapping.split(',').collect();
        if keys.len() > max_size {
            return Err(anyhow!(
                "Too many keys to map. One key can be mapped to a maximum of {} key presses",
                max_size
            ));
        }

        // check delay
        if max_size == consts::MAX_KEY_PRESSES_8890 {
            if btn.delay > 0 {
                println!(
                    "Warning - 0x8890 devices do not support the delay feature - delay value [{}] will be ignored", btn.delay
                );
            }
        } else if btn.delay > consts::MAX_DELAY {
            return Err(anyhow!(
                "delay value [{}] must be between 0 and 6000 msec",
                btn.delay
            ));
        }

        // check individual keys
        for (i, k) in keys.iter().enumerate() {
            let single_key: Vec<_> = k.split('-').collect();
            if max_size == consts::MAX_KEY_PRESSES_8890 && i > 0 && single_key.len() > 1 {
                return Err(anyhow!(
                    "0x8890 macropad only supports modifier keys on first key in sequence"
                ));
            }
            for sk in single_key {
                let da_key = Self::uppercase_first(sk);
                // could be media, control, or regular key
                let mut found = false;
                let mut unsupported = "";
                for i in 0..4 {
                    match i {
                        0 => {
                            found = Self::is_modifier_key(&da_key);
                        }
                        1 => {
                            found = Self::is_media_key(&da_key);
                            // 0x8890 does not support keys > 0xff
                            if pid.is_some() && pid.unwrap() == 0x8890 && found {
                                unsupported = match da_key.as_str() {
                                    "Play" | "Previous" | "Next" | "Mute" | "Volumeup"
                                    | "Volumedown" => "",
                                    _ => &da_key,
                                };
                            }
                        }
                        2 => {
                            found = Self::is_regular_key(&da_key);
                        }
                        3 => {
                            found = Self::is_mouse_action(&da_key);
                        }
                        _ => (),
                    }
                    if !unsupported.is_empty() {
                        return Err(anyhow!("unsupported media key"));
                    }
                    if found {
                        break;
                    }
                }
                if !found {
                    return Err(anyhow!("unknown key - {}", sk));
                }
            }
        }
        Ok(())
    }

    fn uppercase_first(data: &str) -> String {
        let mut result = String::new();
        let mut first = true;
        for value in data.chars() {
            if first {
                result.push(value.to_ascii_uppercase());
                first = false;
            } else {
                result.push(value);
            }
        }
        result
    }

    fn is_modifier_key(keystr: &str) -> bool {
        let ck = Modifier::from_str(keystr);
        if ck.is_ok() {
            return true;
        }
        false
    }

    fn is_media_key(keystr: &str) -> bool {
        let mk = MediaCode::from_str(keystr);
        if mk.is_ok() {
            return true;
        }
        false
    }

    fn is_regular_key(keystr: &str) -> bool {
        let rk = WellKnownCode::from_str(keystr);
        if rk.is_ok() {
            return true;
        }
        false
    }

    fn is_mouse_action(keystr: &str) -> bool {
        matches!(
            keystr.to_lowercase().as_str(),
            "wheelup" | "wheeldown" | "click" | "mclick" | "rclick"
        )
    }
}

#[cfg(test)]
mod tests {

    use crate::mapping::Button;
    use crate::{consts, mapping::Mapping};

    #[test]
    fn mapping_read() {
        Mapping::read("./mapping.ron");
    }

    #[test]
    fn mapping_print() {
        Mapping::print(Mapping::read("./mapping.ron"));
    }

    #[test]
    fn mapping_validate() -> anyhow::Result<()> {
        Mapping::validate("./mapping.ron", None)?;
        Ok(())
    }

    #[test]
    fn mapping_mismatch() {
        assert!(Mapping::validate("./mapping.ron", Some(0x8890)).is_err());
    }

    #[test]
    fn bad_delay_884x() {
        assert!(Mapping::validate_key_mapping(
            &Button {
                delay: 6001,
                mapping: "t,e,s,t".to_string()
            },
            consts::MAX_KEY_PRESSES_884X,
            Some(0x8840)
        )
        .is_err());
    }

    #[test]
    fn test_delay() -> anyhow::Result<()> {
        Mapping::validate_key_mapping(
            &Button {
                delay: 6000,
                mapping: "t,e,s,t".to_string(),
            },
            consts::MAX_KEY_PRESSES_884X,
            Some(0x8840),
        )?;
        Mapping::validate_key_mapping(
            &Button {
                delay: 1234,
                mapping: "t,e,s,t".to_string(),
            },
            consts::MAX_KEY_PRESSES_8890,
            Some(0x8890),
        )?;
        Ok(())
    }

    #[test]
    fn mapping_multiple_modifiers_8890() {
        assert!(Mapping::validate_key_mapping(
            &Button {
                delay: 0,
                mapping: "ctrl-a,shift-s".to_string()
            },
            consts::MAX_KEY_PRESSES_8890,
            Some(0x8890)
        )
        .is_err());
        assert!(Mapping::validate_key_mapping(
            &Button {
                delay: 0,
                mapping: "alt-a,ctrl-s".to_string()
            },
            consts::MAX_KEY_PRESSES_8890,
            Some(0x8890)
        )
        .is_err());
        assert!(Mapping::validate_key_mapping(
            &Button {
                delay: 0,
                mapping: "shift-a,alt-s".to_string()
            },
            consts::MAX_KEY_PRESSES_8890,
            Some(0x8890)
        )
        .is_err());
    }

    #[test]
    fn mapping_max_size_8890() -> anyhow::Result<()> {
        Mapping::validate_key_mapping(
            &Button {
                delay: 0,
                mapping: "1,2,3,4,5".to_string(),
            },
            consts::MAX_KEY_PRESSES_8890,
            Some(0x8890),
        )?;
        assert!(Mapping::validate_key_mapping(
            &Button {
                delay: 0,
                mapping: "1,2,3,4,5,6".to_string()
            },
            consts::MAX_KEY_PRESSES_8890,
            Some(0x8890)
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn mapping_multiple_modifiers_8840() -> anyhow::Result<()> {
        Mapping::validate_key_mapping(
            &Button {
                delay: 0,
                mapping: "ctrl-a,shift-s".to_string(),
            },
            consts::MAX_KEY_PRESSES_884X,
            Some(0x8840),
        )?;
        Ok(())
    }

    #[test]
    fn mapping_max_size_8840() -> anyhow::Result<()> {
        Mapping::validate_key_mapping(
            &Button {
                delay: 0,
                mapping: "1,2,3,4,5,6,7,8,9,0,a,b,c,d,e,f,g".to_string(),
            },
            consts::MAX_KEY_PRESSES_884X,
            Some(0x8840),
        )?;
        assert!(Mapping::validate_key_mapping(
            &Button {
                delay: 0,
                mapping: "1,2,3,4,5,6,7,8,9,0,a,b,c,d,e,f,g,h".to_string()
            },
            consts::MAX_KEY_PRESSES_884X,
            Some(0x8840)
        )
        .is_err());
        Ok(())
    }
}
