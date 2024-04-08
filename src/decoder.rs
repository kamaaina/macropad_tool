use crate::keyboard::{MediaCode, Modifier, Modifiers, WellKnownCode};
use num::FromPrimitive;

pub struct Decoder {}

struct KeyCode {
    #[allow(unused)]
    modifier: u8,
    #[allow(unused)]
    media_code: Option<MediaCode>,
    #[allow(unused)]
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
    pub keys: Vec<u8>,
}

impl Decoder {
    pub fn get_device_info(buf: &[u8]) -> DeviceInformation {
        DeviceInformation {
            num_keys: buf[2],
            num_encoders: buf[3],
        }
    }

    pub fn get_key_mapping(buf: &[u8]) -> KeyMapping {
        let mut key_press: Vec<Option<KeyCode>> = Vec::new();
        let _is_mouse = buf[10] & 0x04; // FIXME: implement
        let mut i = 11;
        loop {
            let val = Self::get_key(&[buf[i], buf[i + 1]]);
            if val.is_none() {
                println!("val is None");
                break; // short circuit
            }

            // TODO: get the mapping
            println!("==> {:?}", &val.unwrap().modifier);
            //key_press.push(val);

            i += 2;
            if i > 45 {
                break; // end of mapping space in usb response
            }
        }
        KeyMapping {
            delay: u16::from_be_bytes([buf[5], buf[6]]),
            layer: buf[3],
            key_number: buf[2],
            keys: Vec::new(),
        }
    }

    fn get_key(buf: &[u8]) -> Option<KeyCode> {
        let val = u16::from_be_bytes([buf[0], buf[1]]);
        println!("val: 0x{:02x}", val);
        if val == 0 {
            return None;
        }

        // get the key/mouse combination
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

        /*
        println!("foo: {} len: {}", foo, foo.len());
        let mut need_dash = false;
        for i in foo {
            if need_dash {
                retval += "-";
            }
            retval += &i.to_string();
            need_dash = true;
        }
        println!("modifier: {retval}");
        */
        println!("modifier: {retval}");
        retval
    }
}

// enum stuff - https://enodev.fr/posts/rusticity-convert-an-integer-to-an-enum.html

// macropad device probe response - 6 button 1 encoder
// 03fb060100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

// layer 1
// key 1 = ctrl+a
// 03fa010101000000000001010400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 2 = alt+shift
// 03fa020101000000000001060000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 3 = ctrl+alt+b
// 03fa030101000000000002050001050000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 4 = null
// 03fa040101000000000001006400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 5 = k
// 03fa050101000000000001000e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 6 = l
// 03fa060101000000000001000f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 16 = mouse wheel +
// 03fa100103000000000004000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 17 = mouse left click
// 03fa110103000000000004000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
// key 18 = mouse wheel -
// 03fa12010300000000000400000000ff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

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

#[cfg(test)]
mod tests {

    use crate::decoder::Decoder;
    use crate::keyboard::{Modifier, Modifiers};

    // cargo test -- --nocapture

    #[test]
    fn foo() {
        let x: Modifiers = Modifier::Alt | Modifier::Ctrl;
        println!("x: {}", x);
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
    fn decode_key() {
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
        let mut key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);
        println!("{:?}", key);

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
        key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);
        println!("{:?}", key);

        // layer = 1
        // key = 3
        // mapping = ctrl+alt+b
        msg = vec![
            0x03, 0xfa, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x05, 0x00, 0x01,
            0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 3");
        key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);
        println!("{:?}", key);

        /*
        //==============================

        // layer = 1
        // key = 4
        // mapping = null
        msg = vec![
            0x03, 0xfa, 0x04, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x64, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 4");
        key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);

        // layer = 1
        // key = 5
        // mapping = k
        msg = vec![
            0x03, 0xfa, 0x05, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x0e, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        println!("\ntest 5");
        key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);

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
        key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);

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
        key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);

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
        key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);

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
        key = Decoder::get_key_mapping(&msg);
        assert_eq!(key.layer, 1);
        */
    }
}
