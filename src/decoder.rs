use crate::keyboard::{MediaCode, WellKnownCode};
use anyhow::{anyhow, Result};
use log::debug;
use num::FromPrimitive;

pub struct Decoder {}

#[derive(Debug)]
pub struct KeyCode {
    modifier: u8,
    media_code: Option<MediaCode>,
    wkc: Option<WellKnownCode>,
}

/// Macropad information
pub struct DeviceInformation {
    /// Number of keys on the macropad
    pub num_keys: u8,
    /// Number of rotary encoders on the macropad
    pub num_encoders: u8,
}

/// Mapping of a key/encoder for the device
#[derive(Debug)]
pub struct KeyMapping {
    /// Delay value which is used for msec delay between key presses
    /// Valid values are 0-6000 inclusive
    pub delay: u16,
    /// Layer to program
    pub layer: u8,
    /// Key index on layer to program
    pub key_number: u8,
    /// Essentially the keychord for the key
    pub keys: Vec<String>,
}

impl Decoder {
    pub fn get_device_info(buf: &[u8]) -> DeviceInformation {
        DeviceInformation {
            num_keys: buf[2],
            num_encoders: buf[3],
        }
    }

    pub fn get_key_mapping(buf: &[u8]) -> Result<KeyMapping> {
        if buf[1] != 0xfa {
            return Err(anyhow!(
                "Message does not appear to be a response from device"
            ));
        }

        let mut key_press: Vec<String> = Vec::new();
        let mut i = 11;

        // check for type of key in byte 4
        // 0x01 - WellKnownKey
        // 0x02 - Multimedia
        // 0x03 - Mouse

        // can we do this or should we check if bit 0 and bit 1 is set?
        if buf[4] == 0x03 {
            // mouse wheel
            let mut wheel_mapping = String::new();
            if buf[10] == 0x04 {
                let mod_key = Self::get_key(&[buf[11], buf[12]]);
                if mod_key.is_some() {
                    let result = mod_key.unwrap();
                    wheel_mapping = Self::modifier_to_str(result.modifier);
                }
            } else if buf[10] == 0x01 {
            }

            // mouse click
            let mut click_type = wheel_mapping;
            if !click_type.is_empty() {
                click_type += "-";
            }
            match buf[12] {
                0x01 => click_type = "click".to_string(),
                0x02 => click_type = "rclick".to_string(),
                0x04 => click_type = "mclick".to_string(),
                _ => (),
            }

            // mouse wheel status
            if click_type.ends_with('-') {
                click_type.pop();
            }
            let mut key_str = click_type;
            match buf[15] {
                0x01 => {
                    if !key_str.is_empty() {
                        key_str += "-";
                    }
                    key_str += "wheelup";
                }
                0xFF => {
                    if !key_str.is_empty() {
                        key_str += "-";
                    }
                    key_str += "wheeldown";
                }
                _ => (),
            }
            key_press.push(key_str);

            // TODO: is it possible to make a binding like wheelup-a? doesn't make much sense
            //       but might need to add support for that. currently, not supported

            return Ok(KeyMapping {
                delay: u16::from_be_bytes([buf[5], buf[6]]),
                layer: buf[3],
                key_number: buf[2],
                keys: key_press,
            });
        } // end buf[4] == 0x03 (Mouse)

        // Multimedia
        if buf[4] == 0x02 {
            let mut tmp = vec![0u8, 2];
            tmp[1] = buf[i];

            let val = Self::get_key(&tmp);

            let result = val.unwrap();
            //println!("result: {:?}", result);
            let mut key_str = Self::modifier_to_str(result.modifier);
            if result.media_code.is_some() {
                if !key_str.is_empty() {
                    key_str += "-";
                }
                key_str += &result.media_code.unwrap().to_string();
            }
            key_press.push(key_str);
            i += 1;
        } // end buf[4] == 0x02 (Multimedia)

        loop {
            let val = Self::get_key(&[buf[i], buf[i + 1]]);
            if val.is_none() {
                break;
            }

            // get the mapping
            let result = val.unwrap();
            let mut key_str = Self::modifier_to_str(result.modifier);
            if result.wkc.is_some() {
                //println!("WKC!!!!");
                if !key_str.is_empty() {
                    key_str += "-";
                }
                key_str += &result.wkc.unwrap().to_string();
            }
            key_press.push(key_str);

            i += 2;
            if i > 45 {
                break; // end of mapping space in usb response
            }
        }
        Ok(KeyMapping {
            delay: u16::from_be_bytes([buf[5], buf[6]]),
            layer: buf[3],
            key_number: buf[2],
            keys: key_press,
        })
    }

    fn get_key(buf: &[u8]) -> Option<KeyCode> {
        let val = u16::from_be_bytes([buf[0], buf[1]]);
        debug!("val: 0x{:02x}", val);
        if val == 0 {
            return None;
        }

        // get the key combination
        let mut da_key = None;
        let mut mc_key = None;
        if buf[1] > 0 {
            da_key = Some(<WellKnownCode as FromPrimitive>::from_u8(buf[1]))?;
            mc_key = Some(<MediaCode as FromPrimitive>::from_u8(buf[1]))?;
        }

        Some(KeyCode {
            modifier: buf[0],
            media_code: mc_key,
            wkc: da_key,
        })
    }

    pub fn modifier_to_str(modifier: u8) -> String {
        let mut retval = Vec::new();
        for i in 0..=7 {
            if modifier >> i & 1 == 1 {
                match i {
                    0 => {
                        retval.push("ctrl");
                    }
                    1 => {
                        retval.push("shift");
                    }
                    2 => {
                        retval.push("alt");
                    }
                    3 => {
                        retval.push("win");
                    }
                    4 => {
                        retval.push("rctrl");
                    }
                    5 => {
                        retval.push("rshift");
                    }
                    6 => {
                        retval.push("ralt");
                    }
                    _ => {
                        break;
                    }
                }
            }
        }

        retval.join("-")
    }
}

#[cfg(test)]
mod tests {

    use crate::decoder::Decoder;
    use anyhow::Result;

    #[test]
    fn modifier_test() {
        assert_eq!(Decoder::modifier_to_str(0x06), "shift-alt");
        assert_eq!(Decoder::modifier_to_str(0x60), "rshift-ralt");
    }

    #[test]
    fn decode_device() {
        // response for a 6 button 1 rotary encoder macropad
        let device = vec![
            0x03, 0xfb, 0x06, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mp = Decoder::get_device_info(&device);
        assert_eq!(mp.num_keys, 6);
        assert_eq!(mp.num_encoders, 1);
    }

    #[test]
    fn decode_key() -> Result<()> {
        env_logger::init();
        // layer = 1
        // key = 1
        // mapping = ctrl+a
        let mut msg = vec![
            0x03, 0xfa, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x04, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("test 1");
        let mut key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys.len(), 1);
        assert_eq!(key.keys[0], "ctrl-a");

        msg = vec![
            0x03, 0xfa, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 2");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys.len(), 1);
        assert_eq!(key.keys[0], "shift-alt");

        msg = vec![
            0x03, 0xfa, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x05, 0x05, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 3");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys.len(), 1);
        assert_eq!(key.keys[0], "ctrl-alt-b");

        msg = vec![
            0x03, 0xfa, 0x04, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 4");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys.len(), 0);

        msg = vec![
            0x03, 0xfa, 0x05, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x05, 0x0e, 0x05,
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 5");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys.len(), 2);
        assert_eq!(key.keys[0], "ctrl-alt-k");
        assert_eq!(key.keys[1], "ctrl-alt-a");

        msg = vec![
            0x03, 0xfa, 0x06, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x0f, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 6");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys[0], "l");

        msg = vec![
            0x03, 0xfa, 0x10, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 7");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys[0], "wheelup");

        msg = vec![
            0x03, 0xfa, 0x11, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 8");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys[0], "click");

        msg = vec![
            0x03, 0xfa, 0x12, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 9");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys[0], "wheeldown");

        msg = vec![
            0x03, 0xfa, 0x13, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 10");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys[0], "ctrl-wheelup");

        msg = vec![
            0x03, 0xfa, 0x13, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00,
            0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 11");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 1);
        println!("{:?}", key);
        assert_eq!(key.keys[0], "ctrl-wheeldown");

        msg = vec![
            0x03, 0xfa, 0x10, 0x03, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xea, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 12");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 3);
        println!("{:?}", key);
        assert_eq!(key.keys[0], "volumedown");

        msg = vec![
            0x03, 0xfa, 0x11, 0x03, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xe2, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 13");
        key = Decoder::get_key_mapping(&msg)?;
        assert_eq!(key.layer, 3);
        println!("{:?}", key);
        assert_eq!(key.keys[0], "mute");

        Ok(())
    }
}
