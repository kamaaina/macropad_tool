use crate::{
    consts,
    decoder::{Decoder, KeyMapping},
    keyboard::{
        Configuration, Keyboard, LedColor, MediaCode, Messages, Modifier, MouseAction, MouseButton,
        WellKnownCode,
    },
    mapping::Macropad,
};
use anyhow::{anyhow, ensure, Result};
use log::{debug, info};
use num::ToPrimitive;
use rusb::{Context, DeviceHandle};
use std::str::FromStr;

pub struct Keyboard884x {
    handle: Option<DeviceHandle<Context>>,
    out_endpoint: u8,
    in_endpoint: u8,
}

impl Configuration for Keyboard884x {
    fn read_macropad_config(&mut self, layer: &u8) -> Result<Macropad> {
        let mut buf = vec![0; consts::READ_BUF_SIZE.into()];

        // get the type of device
        let _ = self.send(&self.device_type());
        let _ = self.recieve(&mut buf);
        let device_info = Decoder::get_device_info(&buf);
        info!(
            "OUT: 0x{:02x} IN: 0x{:02x}",
            self.get_out_endpoint(),
            self.get_in_endpoint()
        );
        debug!(
            "number of keys: {} number of rotary encoders: {}",
            device_info.num_keys, device_info.num_encoders
        );

        // send message to get keys and process later so we don't slow the usb traffic
        // not sure if that would be an issue as i don't know the usb protocol. mabye
        // we could process here too??
        let mut mappings: Vec<KeyMapping> = Vec::new();
        if *layer > 0 {
            // specific layer
            let _ = self.send(&self.read_config(
                device_info.num_keys,
                device_info.num_encoders,
                *layer,
            ));
            // read keys for specified layer
            info!("reading keys for layer {}", layer);
            let data = self.read_config(device_info.num_keys, device_info.num_encoders, *layer);
            let _ = self.send(&data);

            // read all messages from device
            loop {
                let bytes_read = self.recieve(&mut buf)?;
                if bytes_read == 0 {
                    break;
                }
                debug!("bytes read: {bytes_read}");
                debug!("data: {:02x?}", buf);
                mappings.push(Decoder::get_key_mapping(&buf)?);
            }
        } else {
            // read keys for all layers
            for i in 1..=consts::NUM_LAYERS {
                let _ =
                    self.send(&self.read_config(device_info.num_keys, device_info.num_encoders, i));
                info!("reading keys for layer {i}");
                let data = self.read_config(device_info.num_keys, device_info.num_encoders, i);
                let _ = self.send(&data);

                // read all messages from device
                loop {
                    let bytes_read = self.recieve(&mut buf)?;
                    if bytes_read == 0 {
                        break;
                    }
                    debug!("bytes read: {bytes_read}");
                    debug!("data: {:02x?}", buf);
                    mappings.push(Decoder::get_key_mapping(&buf)?);
                }
            }
        }

        // process responses from device
        let rows_cols = Self::guestimate_rows_cols(device_info.num_keys)?;
        let mut mp = Macropad::new(rows_cols.0, rows_cols.1, device_info.num_encoders);
        let mut knob_idx = 0;
        let mut knob_type = 0;
        let mut last_layer = 0;
        for km in mappings {
            debug!("{:?}", km);
            if km.layer != last_layer {
                last_layer = km.layer;
                knob_idx = 0;
                knob_type = 0;
            }

            if km.key_number <= mp.device.rows * mp.device.cols {
                // button mappings
                let row_col = Self::get_position(&mp, km.key_number)?;
                debug!(
                    "   key: {} at row: {} col: {}",
                    km.key_number, row_col.0, row_col.1
                );
                mp.layers[(km.layer - 1) as usize].buttons[row_col.0][row_col.1] =
                    km.keys.join(",");
            } else {
                // knobs
                debug!("knob idx: {} knob type: {}", knob_idx, knob_type);
                match knob_type {
                    0 => {
                        mp.layers[(km.layer - 1) as usize].knobs[knob_idx].ccw = km.keys.join("-");
                        knob_type += 1;
                    }
                    1 => {
                        mp.layers[(km.layer - 1) as usize].knobs[knob_idx].press =
                            km.keys.join("-");
                        knob_type += 1;
                    }
                    2 => {
                        mp.layers[(km.layer - 1) as usize].knobs[knob_idx].cw = km.keys.join("-");
                        knob_type = 0;
                        knob_idx += 1;
                    }
                    _ => {
                        panic!("should not get here!")
                    }
                }
            }
        }
        Ok(mp)
    }
}

impl Messages for Keyboard884x {
    fn read_supported(&self) -> bool {
        true
    }

    fn read_config(&self, keys: u8, encoders: u8, layer: u8) -> Vec<u8> {
        vec![
            0x03, 0xfa, keys, encoders, layer, 0x02, 0xe0, 0xcb, 0x80, 0x00, 0xa0, 0xcc, 0x80,
            0x00, 0x7c, 0xf2, 0x02, 0x69, 0x00, 0x00, 0x00, 0x00, 0x4d, 0x00, 0x2c, 0x02, 0xa0,
            0xcc, 0x80, 0x00, 0xe8, 0x00, 0x00, 0x00, 0xb9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x90, 0xcc, 0x80, 0x00, 0x20, 0xcd, 0x80, 0x00, 0xc0, 0x84, 0x26, 0x02, 0xa0,
            0x62, 0x2f, 0x02, 0xc0, 0xcc, 0x80, 0x00, 0xc7, 0xb6, 0xc2,
        ]
    }

    fn device_type(&self) -> Vec<u8> {
        vec![
            0x03, 0xfb, 0xfb, 0xfb, 0x1f, 0x02, 0x3c, 0xd0, 0x80, 0x00, 0xec, 0xcf, 0x80, 0x00,
            0xcc, 0xd2, 0x9b, 0x00, 0xf0, 0xcf, 0x80, 0x00, 0x3c, 0xd0, 0x80, 0x00, 0x56, 0x83,
            0xd2, 0x7b, 0xd0, 0x0d, 0x48, 0x00, 0x0c, 0xd0, 0x80, 0x00, 0xa8, 0x3d, 0x34, 0x02,
            0x48, 0xd0, 0x80, 0x00, 0x70, 0xf5, 0x1e, 0x62, 0x98, 0xda, 0x11, 0x62, 0x0c, 0x80,
            0x00, 0x00, 0x00, 0x82, 0x26, 0x02, 0xff, 0xff, 0xff,
        ]
    }

    fn program_led(&self, mode: u8, color: LedColor) -> Vec<u8> {
        let mut m_c = <LedColor as ToPrimitive>::to_u8(&color).unwrap();
        m_c |= mode;
        debug!("mode and code:0x{:02}", m_c);
        let mut msg = vec![0x03, 0xfe, 0xb0, 0x01, 0x08];
        msg.extend_from_slice(&[0; 5]);
        msg.extend_from_slice(&[0x01, 0x00, m_c]);
        msg.extend_from_slice(&[0; 52]);
        msg
    }

    fn end_program(&self) -> Vec<u8> {
        let mut msg = vec![0x03, 0xfd, 0xfe, 0xff];
        msg.extend_from_slice(&[0; 61]);
        msg
    }
}

impl Keyboard for Keyboard884x {
    fn program(&mut self, macropad: &Macropad) -> Result<()> {
        // ensure the config we have matches the connected device we want to program
        let mut buf = vec![0; consts::READ_BUF_SIZE.into()];

        // get the type of device
        let _ = self.send(&self.device_type());
        let _ = self.recieve(&mut buf);
        let device_info = Decoder::get_device_info(&buf);
        ensure!(
                device_info.num_keys == (macropad.device.rows * macropad.device.cols)
                    && device_info.num_encoders == macropad.device.knobs,
                "Configuration file and macropad mismatch.\nLooks like you are trying to program a different macropad.\nDid you select the right configuration file?\n"
            );

        for (i, layer) in macropad.layers.iter().enumerate() {
            let lyr = (i + 1) as u8;
            let mut j = 1;
            for row in &layer.buttons {
                for btn in row {
                    debug!("program layer: {} key: 0x{:02x} to: {btn}", i + 1, j);
                    self.send(&self.build_key_msg(btn.to_string(), lyr, j, 0)?)?;
                    j += 1;
                }
            }

            // TODO: test 9x3 to see if the 3 knobs are top to bottom with key number
            j = 0x10;
            for knob in &layer.knobs {
                debug!("layer: {} key: 0x{:02x} knob cw {}", i + 1, j, knob.cw);
                self.send(&self.build_key_msg(knob.cw.clone(), lyr, j, 0)?)?;
                j += 1;

                debug!(
                    "layer: {} key: 0x{:02x} knob press {}",
                    i + 1,
                    j,
                    knob.press
                );
                self.send(&self.build_key_msg(knob.press.clone(), lyr, j, 0)?)?;
                j += 1;

                debug!("layer: {} key: 0x{:02x} knob ccw {}", i + 1, j, knob.ccw);
                self.send(&self.build_key_msg(knob.ccw.clone(), lyr, j, 0)?)?;
                j += 1;
            }
        }
        let _ = self.send(&self.end_program());
        Ok(())
    }

    fn set_led(&mut self, mode: u8, color: LedColor) -> Result<()> {
        self.send(&self.program_led(mode, color))?;
        self.send(&self.end_program())?;
        Ok(())
    }

    fn get_handle(&self) -> &DeviceHandle<Context> {
        self.handle.as_ref().unwrap()
    }

    fn get_out_endpoint(&self) -> u8 {
        self.out_endpoint
    }

    fn get_in_endpoint(&self) -> u8 {
        self.in_endpoint
    }
}

impl Keyboard884x {
    pub fn new(
        handle: Option<DeviceHandle<Context>>,
        out_endpoint: u8,
        in_endpoint: u8,
    ) -> Result<Self> {
        let keyboard = Self {
            handle,
            out_endpoint,
            in_endpoint,
        };

        Ok(keyboard)
    }

    fn build_key_msg(
        &self,
        key_chord: String,
        layer: u8,
        key_pos: u8,
        _delay: u16,
    ) -> Result<Vec<u8>> {
        let keys: Vec<_> = key_chord.split(',').collect();
        let mut msg = vec![0x03, 0xfd, key_pos, layer, 0x01];
        msg.extend_from_slice(&[0; 5]);
        msg.extend_from_slice(&[keys.len().try_into()?]);

        let mut cnt = 0;
        let mut mouse_action = 0u8;
        let mut mouse_click = 0u8;
        for binding in &keys {
            let kc: Vec<_> = binding.split('-').collect();
            let mut m_c = 0x00u8;
            let mut wkk = 0x00;
            for key in kc {
                debug!("=> {key}");
                if let Ok(m) = Modifier::from_str(key) {
                    let power = <Modifier as ToPrimitive>::to_u8(&m).unwrap();
                    m_c = 2u32.pow(power as u32) as u8;
                } else if let Ok(w) = WellKnownCode::from_str(key) {
                    wkk = <WellKnownCode as ToPrimitive>::to_u8(&w).unwrap();
                } else if let Ok(a) = MediaCode::from_str(key) {
                    m_c = <MediaCode as ToPrimitive>::to_u8(&a).unwrap();
                    msg[4] = 0x02;
                    msg[10] = 0x02;
                } else if let Ok(a) = MouseButton::from_str(key) {
                    mouse_click =
                        2u32.pow(<MouseButton as ToPrimitive>::to_u8(&a).unwrap().into()) as u8;
                    msg[4] = 0x03;
                } else if let Ok(a) = MouseAction::from_str(key) {
                    m_c = 0x01;
                    match a {
                        MouseAction::WheelUp => mouse_action = 0x01,
                        MouseAction::WheelDown => mouse_action = 0xff,
                        _ => (),
                    }
                    msg[4] = 0x03;
                }
            }
            msg.extend_from_slice(&[m_c, wkk]);
            cnt += 1;
        }

        for _i in 0..=(consts::MAX_KEY_PRESSES_884X - cnt) {
            msg.extend_from_slice(&[0x00; 2]);
        }

        if mouse_click > 0 {
            msg[12] = mouse_click;
        }
        if mouse_action > 0 {
            msg[15] = mouse_action;
        }

        // last 18 bytes are always 0
        msg.extend_from_slice(&[0; 18]);

        Ok(msg)
    }

    fn get_position(mp: &Macropad, key_num: u8) -> Result<(usize, usize)> {
        let cols = mp.device.cols;
        let mut col;
        let mut row;

        if key_num % cols == 0 {
            row = key_num / cols;
            row = row.saturating_sub(1);
        } else {
            row = key_num / cols;
        }
        if key_num > cols {
            col = key_num % cols;
            if col == 0 {
                col = cols;
            }
            col -= 1;
        } else {
            col = key_num - 1;
        }
        Ok((row.into(), col.into()))
    }

    fn guestimate_rows_cols(num_keys: u8) -> Result<(u8, u8)> {
        match num_keys {
            6 => Ok((2, 3)),
            9 => Ok((3, 3)),
            12 => Ok((3, 4)),
            _ => Err(anyhow!("unable to guess rows/cols for {num_keys}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::keyboard::k884x::Keyboard884x;

    #[test]
    fn ctrl_a_ctrl_s() -> anyhow::Result<()> {
        // ctrl-a,ctrl-s
        // 03 fd 01 01 01 00 00 00     00 00 02 01 04 01 16 00   00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("ctrl-a,ctrl-s".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[10], 0x02, "checking number of keys to program");
        assert_eq!(msg[11], 0x01, "checking for ctrl modifier");
        assert_eq!(msg[12], 0x04, "checking for 'a' key");
        assert_eq!(msg[13], 0x01, "checking for ctrl modifier");
        assert_eq!(msg[14], 0x16, "checking for 's' key");
        Ok(())
    }

    #[test]
    fn well_known_key() -> anyhow::Result<()> {
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("a".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[10], 0x01, "checking number of keys to program");
        assert_eq!(msg[11], 0x00, "checking for modifier");
        assert_eq!(msg[12], 0x04, "checking for 'a' key");
        Ok(())
    }

    #[test]
    fn volume_down() -> anyhow::Result<()> {
        // 03 fd 10 01 02 00 00 00     00 00 02 ea 0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("volumedown".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[4], 0x02, "checking byte 4");
        for i in 5..=9 {
            assert_eq!(msg[i], 0x00);
        }
        assert_eq!(msg[10], 0x02, "checking byte 10");
        assert_eq!(msg[11], 0xea, "checking byte 11");
        Ok(())
    }

    #[test]
    fn mouse_ctrl_plus() -> anyhow::Result<()> {
        // 03 fd 01 02 03 00 00 00     00 00 01 01 00 00 00 01 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("ctrl-wheelup".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[4], 0x03, "checking byte 4");
        for i in 5..=9 {
            assert_eq!(msg[i], 0x00);
        }
        assert_eq!(msg[10], 0x01, "checking byte 10");
        assert_eq!(msg[11], 0x01, "checking byte 11");
        assert_eq!(msg[15], 0x01, "checking byte 15");
        Ok(())
    }

    #[test]
    fn mouse_ctrl_minus() -> anyhow::Result<()> {
        // 03 fd 02 02 03 00 00 00     00 00 01 01 00 00 00 ff 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("ctrl-wheeldown".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[4], 0x03, "checking byte 4");
        for i in 5..=9 {
            assert_eq!(msg[i], 0x00);
        }
        assert_eq!(msg[10], 0x01, "checking byte 10");
        assert_eq!(msg[11], 0x01, "checking byte 11");
        assert_eq!(msg[15], 0xff, "checking byte 15");
        Ok(())
    }

    #[test]
    fn mouse_left_click() -> anyhow::Result<()> {
        // 03 fd 01 02 03 00 00 00     00 00 01 00 01 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("click".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[4], 0x03, "checking byte 4");
        for i in 5..=9 {
            assert_eq!(msg[i], 0x00);
        }
        assert_eq!(msg[10], 0x01, "checking byte 10");
        assert_eq!(msg[11], 0x00, "checking byte 11");
        assert_eq!(msg[12], 0x01, "checking byte 12");
        Ok(())
    }

    #[test]
    fn mouse_middle_click() -> anyhow::Result<()> {
        // 03 fd 02 02 03 00 00 00     00 00 01 00 04 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("mclick".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[4], 0x03, "checking byte 4");
        for i in 5..=9 {
            assert_eq!(msg[i], 0x00);
        }
        assert_eq!(msg[10], 0x01, "checking byte 10");
        assert_eq!(msg[11], 0x00, "checking byte 11");
        assert_eq!(msg[12], 0x04, "checking byte 12");
        Ok(())
    }

    #[test]
    fn mouse_right_click() -> anyhow::Result<()> {
        // 03 fd 03 02 03 00 00 00     00 00 01 00 02 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("rclick".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[4], 0x03, "checking byte 4");
        for i in 5..=9 {
            assert_eq!(msg[i], 0x00);
        }
        assert_eq!(msg[10], 0x01, "checking byte 10");
        assert_eq!(msg[11], 0x00, "checking byte 11");
        assert_eq!(msg[12], 0x02, "checking byte 12");
        Ok(())
    }

    #[test]
    fn shift_p() -> anyhow::Result<()> {
        // 03 fd 06 01 01 00 00 00      00 00 01 02 13 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("shift-p".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[4], 0x01, "checking byte 4");
        for i in 5..=9 {
            assert_eq!(msg[i], 0x00);
        }
        assert_eq!(msg[10], 0x01, "checking byte 10");
        assert_eq!(msg[11], 0x02, "checking byte 11");
        assert_eq!(msg[12], 0x13, "checking byte 12");
        Ok(())
    }

    #[test]
    fn win_enter() -> anyhow::Result<()> {
        // 03 fd 11 03 01 00 00 00      00 00 01 08 28 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
        let kbd = Keyboard884x::new(None, 0, 0)?;
        let msg = kbd.build_key_msg("win-enter".to_string(), 1u8, 1u8, 0)?;
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[4], 0x01, "checking byte 4");
        for i in 5..=9 {
            assert_eq!(msg[i], 0x00);
        }
        assert_eq!(msg[10], 0x01, "checking byte 10");
        assert_eq!(msg[11], 0x08, "checking byte 11");
        assert_eq!(msg[12], 0x28, "checking byte 12");
        Ok(())
    }
}
