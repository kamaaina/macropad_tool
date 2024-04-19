use crate::keyboard::{MediaCode, Modifier, Modifiers, MouseAction, WellKnownCode};
use anyhow::{anyhow, Result};
use num::FromPrimitive;

pub struct Decoder {}

pub struct KeyCode {
    modifier: u8,
    media_code: Option<MediaCode>,
    wkc: Option<WellKnownCode>,
}

pub struct DeviceInformation {
    pub num_keys: u8,
    pub num_encoders: u8,
}

#[derive(Debug)]
pub struct KeyMapping {
    pub delay: u16,
    pub layer: u8,
    pub key_number: u8,
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
            /*
            let mut result: KeyCode;
            let val = Self::get_key(&[buf[10], buf[11]]);
            if val.is_some() {
                result = val.unwrap();
                println!("mouse modifier value: {:?}", &result.modifier);
            }
            let mut key_str = Self::modifier_to_str(result.modifier);
            */

            // mouse wheel
            let mut wheel_mapping = String::new();
            if buf[10] == 0x04 {
                let mod_key = Self::get_key(&[buf[11], buf[12]]);
                if mod_key.is_some() {
                    let result = mod_key.unwrap();
                    wheel_mapping = Self::modifier_to_str(result.modifier);
                }
            }

            // mouse click
            let mut click_type = wheel_mapping;
            if click_type.len() > 0 {
                click_type += "-";
            }
            match buf[12] {
                0x01 => click_type = "click".to_string(),
                0x02 => click_type = "rclick".to_string(),
                0x03 => click_type = "mclick".to_string(),
                _ => (),
            }

            // mouse wheel status
            if click_type.ends_with('-') {
                click_type.pop();
            }
            let mut key_str = click_type;
            match buf[15] {
                0x01 => {
                    if key_str.len() > 0 {
                        key_str += "-";
                    }
                    key_str += "wheelup";
                }
                0xFF => {
                    if key_str.len() > 0 {
                        key_str += "-";
                    }
                    key_str += "wheeldown";
                }
                _ => (),
            }
            key_press.push(key_str);

            // TODO: is it possible to make a binding like wheelup-a? doesn't make much sense
            //       but might need to add support for that. currently, not supported

            // FIXME: add support for modifier key

            return Ok(KeyMapping {
                delay: u16::from_be_bytes([buf[5], buf[6]]),
                layer: buf[3],
                key_number: buf[2],
                keys: key_press,
            });
        }

        loop {
            let val = Self::get_key(&[buf[i], buf[i + 1]]);
            if val.is_none() {
                break;
            }

            // TODO: get the mapping
            let result = val.unwrap();
            println!("modifier value: {:?}", &result.modifier);
            let mut key_str = Self::modifier_to_str(result.modifier);
            if result.wkc.is_some() {
                //println!("WKC!!!!");
                if key_str.len() > 0 {
                    key_str += "-";
                }
                key_str += &result.wkc.unwrap().to_string();
            }
            println!("### key string: {key_str}");
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
        //println!("val: 0x{:02x}", val);
        if val == 0 {
            return None;
        }

        // get the key combination
        let mut da_key = None;
        if buf[1] > 0 {
            da_key = Some(<WellKnownCode as FromPrimitive>::from_u32(buf[1].into()))?;
        }

        Some(KeyCode {
            modifier: buf[0],
            media_code: None,
            wkc: da_key,
        })
    }

    pub fn modifier_to_str(modifier: u8) -> String {
        let mut retval = String::new();
        for i in 0..=7 {
            if modifier >> i & 1 == 1 {
                match i {
                    0 => {
                        // left ctrl
                        if retval.len() > 0 {
                            retval += "-ctrl";
                        } else {
                            retval += "ctrl";
                        }
                    }
                    1 => {
                        // left shift
                        if retval.len() > 0 {
                            retval += "-shift";
                        } else {
                            retval += "shift";
                        }
                    }
                    2 => {
                        // left alt
                        if retval.len() > 0 {
                            retval += "-alt";
                        } else {
                            retval += "alt";
                        }
                    }
                    3 => {
                        // window key
                        if retval.len() > 0 {
                            retval += "-win";
                        } else {
                            retval += "win";
                        }
                    }
                    4 => {
                        // right ctrl
                        if retval.len() > 0 {
                            retval += "-rctrl";
                        } else {
                            retval += "rctrl";
                        }
                    }
                    5 => {
                        // right shift
                        if retval.len() > 0 {
                            retval += "-rshift";
                        } else {
                            retval += "rshift";
                        }
                    }
                    6 => {
                        // right alt
                        if retval.len() > 0 {
                            retval += "-ralt";
                        } else {
                            retval += "ralt";
                        }
                    }
                    _ => {
                        break;
                    }
                }
            }
        }

        //println!("modifier: {retval}");
        retval
    }
}

// enum stuff - https://enodev.fr/posts/rusticity-convert-an-integer-to-an-enum.html

// macropad device probe response - 6 button 1 encoder
// 03fb060100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

// layer 2
// key 16 = left arrow
// 03fa100201000000000001005000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 17 = enter
// 03fa110201000000000001002800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 18 = right arrow
// 03fa120201000000000001004f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

// layer 3
// key 1 = play/pause
// 03fa010302000000000001cd0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 2 = next track
// 03fa020302000000000001b50000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 3 = mute
// 03fa030302000000000001e20000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 4 = 3
// 03fa040301000000000001002000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 5 = 4
// 03fa050301000000000001002100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 6 = 5
// 03fa060301000000000001002200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 16 = volume -
// 03fa100302000000000001ea0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 17 = mute
// 03fa110302000000000001e20000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 18 = volume +
// 03fa120302000000000001e90000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

// program key
// layer 1 4 keys (abcd)
// 03fd010101000000000004000400050006000700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// read
// 03fa010101000000000004000400050006000700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

// program key
// layer 1 one key (play/pause)
// 03fd020102000000000002cd0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// read
// 03fa020102000000000001cd0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

#[cfg(test)]
mod tests {

    use crate::decoder::Decoder;
    use crate::keyboard::{Modifier, Modifiers};
    use anyhow::Result;
    use num::FromPrimitive;

    // cargo test -- --nocapture

    #[test]
    fn foo() {
        let x: Modifiers = Modifier::Alt | Modifier::Ctrl;
        println!("x: {}", x);
        let val: u8 = 0xff;
        let a = <Modifier as FromPrimitive>::from_u8(val);
        println!("a: {a:?}");
        assert_eq!(a, None);
    }

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

        // layer = 1
        // key = 2
        // mapping = alt+shift
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

        // layer = 1
        // key = 3
        // mapping = ctrl+alt+b
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

        //==============================

        // layer = 1
        // key = 4
        // mapping = null
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

        // layer = 1
        // key = 5
        // mapping = k
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

        // layer = 1
        // key = 6
        // mapping = l
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

        // layer = 1
        // key = 16
        // mapping = mouse wheel +
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

        // layer = 1
        // key = 16
        // mapping = mouse left click
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

        // layer = 1
        // key = 16
        // mapping = mouse wheel -
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

        // ctrl-wheelup
        // 03 fa 13 01 03 00 00 00 00 00 04 01 00 00 00 01 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
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

        // ctrl-wheeldown
        // 03 fa 15 01 03 00 00 00 00 00 04 01 00 00 00 ff 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
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

        Ok(())
    }
}
