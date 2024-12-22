use crate::{
    consts,
    keyboard::{
        Configuration, Keyboard, LedColor, MediaCode, Messages, Modifier, MouseAction, MouseButton,
        WellKnownCode,
    },
    Macropad,
};
use anyhow::{anyhow, Result};
use log::debug;
use num::ToPrimitive;
use rusb::{Context, DeviceHandle};
use std::str::FromStr;

pub struct Keyboard8890 {
    handle: Option<DeviceHandle<Context>>,
    out_endpoint: u8,
    led_programmed: bool,
}

impl Configuration for Keyboard8890 {
    fn read_macropad_config(&mut self, _layer: &u8) -> Result<Macropad> {
        Err(anyhow!("not supported for this macropad"))
    }
}

impl Messages for Keyboard8890 {
    fn read_config(&self, _keys: u8, _encoders: u8, _layer: u8) -> Vec<u8> {
        unimplemented!("reading configuration from this macropad is not supported");
    }

    fn device_type(&self) -> Vec<u8> {
        unimplemented!("reading device type is not supported");
    }

    fn program_led(&self, mode: u8, _layer: u8, _color: LedColor) -> Vec<u8> {
        let mut msg = vec![0x03, 0xb0, 0x18, mode];
        let size = consts::PACKET_SIZE - msg.len();
        msg.extend_from_slice(&vec![0; size]);
        msg
    }

    fn end_program(&self) -> Vec<u8> {
        let last_byte = if self.led_programmed { 0xa1 } else { 0xaa };
        let mut msg = vec![0x03, 0xaa, last_byte];
        let size = consts::PACKET_SIZE - msg.len();
        msg.extend_from_slice(&vec![0; size]);
        msg
    }
}

impl Keyboard for Keyboard8890 {
    fn program(&mut self, macropad: &Macropad) -> Result<()> {
        debug!("programming keyboard - NOTE: hardcoding to layer 1");

        // FIXME: currently hardcoding the layer to 1 as the only 8890 device
        //        i have seen only has support for one layer. if we know of
        //        one that has multiple layers, we should refactor this then
        self.send(&self.begin_programming(1))?;

        // get our layout of buttons relative to programming orientation
        let layout = self.get_layout(
            macropad.device.orientation,
            macropad.device.rows,
            macropad.device.cols,
        )?;
        debug!("layout: {layout:?}");

        for (i, layer) in macropad.layers.iter().enumerate() {
            let mut key_num;
            for (row_idx, row) in layer.buttons.iter().enumerate() {
                for (col_idx, btn) in row.iter().enumerate() {
                    debug!("get position in layout: row_idx: {row_idx} col_idx: {col_idx}");
                    key_num = layout[row_idx][col_idx];
                    debug!(
                        "program layer: {} key: 0x{:02x} to: {btn:?}",
                        i + 1,
                        key_num
                    );
                    let keys: Vec<_> = btn.mapping.split(',').collect();
                    if keys.len() > consts::MAX_KEY_PRESSES_8890 {
                        return Err(anyhow!(
                            "maximum key presses for this macropad is {}",
                            consts::MAX_KEY_PRESSES_8890
                        ));
                    }
                    for msg in self.map_key(btn.mapping.to_string(), key_num)? {
                        self.send(&msg)?;
                    }
                }
            }
            key_num = 0x0du8;
            for knob in &layer.knobs {
                debug!(
                    "programming knob ccw: {} cw: {} push: {}",
                    knob.ccw.mapping, knob.cw.mapping, knob.press.mapping
                );
                let mut btn;
                for i in 0..3 {
                    match i {
                        0 => btn = knob.ccw.clone(),
                        1 => btn = knob.press.clone(),
                        2 => btn = knob.cw.clone(),
                        _ => unreachable!("should not get here"),
                    }
                    let keys: Vec<_> = btn.mapping.split(',').collect();
                    if keys.len() > consts::MAX_KEY_PRESSES_8890 {
                        return Err(anyhow!(
                            "maximum key presses for this macropad is {}",
                            consts::MAX_KEY_PRESSES_8890
                        ));
                    }
                    for msg in self.map_key(btn.mapping.to_string(), key_num)? {
                        self.send(&msg)?;
                    }
                    key_num += 1;
                }
            }
        }
        self.send(&self.end_program())?;
        debug!("DONE - programming keyboard");
        Ok(())
    }

    fn set_led(&mut self, mode: u8, layer: u8, _color: LedColor) -> Result<()> {
        if mode > 2 {
            return Err(anyhow!("macropad supports modes 0, 1, and 2 only"));
        }
        self.led_programmed = true;
        self.send(&self.begin_programming(layer))?;
        self.send(&self.program_led(mode, layer, LedColor::Red))?;
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
        unimplemented!("reading configuration from this macropad is not supported");
    }
}

impl Keyboard8890 {
    pub fn new(handle: Option<DeviceHandle<Context>>, out_endpoint: u8) -> Result<Self> {
        let keyboard = Self {
            handle,
            out_endpoint,
            led_programmed: false,
        };

        Ok(keyboard)
    }

    pub fn begin_programming(&self, layer: u8) -> Vec<u8> {
        let mut msg = vec![0x03, 0xa1, layer];
        let size = consts::PACKET_SIZE - msg.len();
        msg.extend_from_slice(&vec![0; size]);
        msg
    }

    fn map_key(&self, key_chord: String, key_pos: u8) -> Result<Vec<Vec<u8>>> {
        let mut retval = Vec::new();
        let mut prepend = Vec::new();
        let kc: Vec<_> = key_chord.split(',').collect();
        let mut prepended = false;
        for (i, key) in kc.iter().enumerate() {
            let mut msg = vec![0x03, key_pos, 0x00, 0x00, 0x00, 0x00, 0x00];
            let mut remaining = consts::PACKET_SIZE - msg.len();
            let km: Vec<_> = key.split('-').collect();
            let mut mouse_action = 0u8;
            let mut mouse_click;
            let mut media_key = false;
            let mut media_val = 0u8;
            //let mut m_c;
            let mut wkk;
            for mod_key in km {
                debug!("=====> {mod_key}");
                if let Ok(w) = WellKnownCode::from_str(mod_key) {
                    msg[2] = 0x11;
                    msg[3] = kc.len().try_into()?;
                    msg.extend_from_slice(&[0; 3]);
                    remaining -= 3;
                    let mut first_msg = msg.clone();
                    first_msg.extend_from_slice(&vec![0; remaining]);
                    msg[4] = (i + 1).try_into()?;
                    if !prepended {
                        prepend.push(first_msg);
                        prepended = true;
                    }
                    wkk = <WellKnownCode as ToPrimitive>::to_u8(&w).unwrap();
                    msg[6] = wkk;
                } else if let Ok(a) = MediaCode::from_str(mod_key) {
                    let value = <MediaCode as ToPrimitive>::to_u16(&a).unwrap();
                    msg[2] = 0x12;
                    msg[3] = (value & 0xFF) as u8;
                    media_val = ((value & 0xFF00) >> 8) as u8;
                    media_key = true;
                } else if let Ok(a) = MouseButton::from_str(mod_key) {
                    mouse_click =
                        2u32.pow(<MouseButton as ToPrimitive>::to_u8(&a).unwrap().into()) as u8;
                    msg[2] = 0x13;
                    msg[3] = mouse_click;
                } else if let Ok(a) = MouseAction::from_str(mod_key) {
                    match a {
                        MouseAction::WheelUp => mouse_action = 0x01,
                        MouseAction::WheelDown => mouse_action = 0xff,
                        _ => (),
                    }
                    msg[2] = 0x13;
                    msg[6] = mouse_action;
                } else {
                    // modifier combo (eg. shift-m)
                    let mapping = Keyboard8890::key_mapping(mod_key)?;
                    msg[2] = 0x11;
                    msg[3] = kc.len().try_into()?;
                    msg.extend_from_slice(&[0; 3]);
                    remaining -= 3;
                    let mut first_msg = msg.clone();
                    first_msg.extend_from_slice(&vec![0; remaining]);
                    msg[4] = (i + 1).try_into()?;
                    if !prepended {
                        prepend.push(first_msg);
                        prepended = true;
                    }
                    msg[5] |= mapping.0;
                    msg[6] = mapping.1;
                }
            }
            msg.extend_from_slice(&vec![0; remaining]);
            for i in &prepend {
                retval.push(i.clone());
            }
            if media_key {
                msg[4] = media_val;
            }
            prepend.clear();
            retval.push(msg);
        }

        Ok(retval)
    }

    fn key_mapping(key: &str) -> Result<(u8, u8)> {
        let mut mc = 0;
        let mut wkk = 0;
        let values: Vec<_> = key.split('-').collect();
        for i in values {
            if let Ok(w) = WellKnownCode::from_str(i) {
                wkk = <WellKnownCode as ToPrimitive>::to_u8(&w).unwrap();
            }
            if let Ok(m) = Modifier::from_str(i) {
                let power = <Modifier as ToPrimitive>::to_u8(&m).unwrap();
                mc = 2u32.pow(power as u32) as u8;
            }
        }
        Ok((mc, wkk))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        consts,
        keyboard::{k8890::Keyboard8890, LedColor, Messages},
    };

    #[test]
    fn test_hello() -> anyhow::Result<()> {
        let kbd = Keyboard8890::new(None, 0)?;
        let msgs = kbd.map_key("h,e,l,l,o".to_string(), 4)?;
        println!("{:02x?}", msgs);
        assert_eq!(msgs.len(), 6, "number of messages created");
        for i in msgs.iter().take(6) {
            assert_eq!((*i).len(), consts::PACKET_SIZE, "checking msg size");
        }

        let expected = vec![0x03, 0x04, 0x11, 0x05, 0x00, 0x00];
        assert_eq!(msgs[0].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[0][..6], "checking message");

        let expected = vec![0x03, 0x04, 0x11, 0x05, 0x01, 0x00, 0x0b];
        assert_eq!(msgs[1].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[1][..7], "checking message");

        let expected = vec![0x03, 0x04, 0x11, 0x05, 0x02, 0x00, 0x08];
        assert_eq!(msgs[1].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[2][..7], "checking message");

        let expected = vec![0x03, 0x04, 0x11, 0x05, 0x03, 0x00, 0x0f];
        assert_eq!(msgs[1].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[3][..7], "checking message");

        let expected = vec![0x03, 0x04, 0x11, 0x05, 0x04, 0x00, 0x0f];
        assert_eq!(msgs[1].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[4][..7], "checking message");

        let expected = vec![0x03, 0x04, 0x11, 0x05, 0x05, 0x00, 0x12];
        assert_eq!(msgs[1].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[5][..7], "checking message");

        Ok(())
    }

    #[test]
    fn ctrl_a_ctrl_s() -> anyhow::Result<()> {
        let kbd = Keyboard8890::new(None, 0)?;
        let msgs = kbd.map_key("ctrl-a,ctrl-s".to_string(), 3)?;
        println!("{:02x?}", msgs);
        for i in msgs.iter().take(3) {
            assert_eq!((*i).len(), consts::PACKET_SIZE, "checking msg size");
        }
        assert_eq!(msgs.len(), 3, "number of messages created");

        let expected = vec![0x03, 0x03, 0x11, 0x02, 0x00, 0x00];
        assert_eq!(msgs[0].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[0][..6], "checking message");

        let expected = vec![0x03, 0x03, 0x11, 0x02, 0x01, 0x01, 0x04];
        assert_eq!(msgs[1].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[1][..7], "checking message");

        let expected = vec![0x03, 0x03, 0x11, 0x02, 0x02, 0x01, 0x16];
        assert_eq!(msgs[2].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[2][..7], "checking message");
        Ok(())
    }

    #[test]
    fn a_key() -> anyhow::Result<()> {
        let kbd = Keyboard8890::new(None, 0)?;
        let msgs = kbd.map_key("a".to_string(), 1)?;
        println!("{:02x?}", msgs);
        assert_eq!(msgs.len(), 2, "number of messages created");
        let expected = vec![0x03, 0x01, 0x11, 0x01, 0x00, 0x00];
        assert_eq!(msgs[0].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[0][..6], "checking message");

        let expected = vec![0x03, 0x01, 0x11, 0x01, 0x01, 0x00, 0x04];
        assert_eq!(msgs[1].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[1][..7], "checking message");
        Ok(())
    }

    #[test]
    fn led_mode2() -> anyhow::Result<()> {
        let mut kbd = Keyboard8890::new(None, 0)?;
        kbd.led_programmed = true;
        let msg = kbd.program_led(2, 1, LedColor::Red);
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(msg[1], 0xb0, "checking first byte of led programming");
        assert_eq!(msg[2], 0x18, "checking second byte of led programming");
        assert_eq!(msg[3], 0x02, "checking led mode");
        let msg = kbd.end_program();
        assert_eq!(msg.len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(msg[0], 0x03, "checking first byte of end programming led");
        assert_eq!(msg[1], 0xaa, "checking second byte of end programming led");
        assert_eq!(msg[2], 0xa1, "checking third byte of end programming led");
        Ok(())
    }

    #[test]
    fn end_programming() -> anyhow::Result<()> {
        let kbd = Keyboard8890::new(None, 0)?;
        let msg = kbd.end_program();
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(msg[0], 0x03, "checking first byte of end programming");
        assert_eq!(msg[1], 0xaa, "checking second byte of end programming");
        assert_eq!(msg[2], 0xaa, "checking third byte of end programming");
        Ok(())
    }

    #[test]
    fn volume_up() -> anyhow::Result<()> {
        // 03 01 12 e9 000000...
        let kbd = Keyboard8890::new(None, 0)?;
        let msgs = kbd.map_key("volumeup".to_string(), 1)?;
        println!("{:02x?}", msgs);
        assert_eq!(msgs.len(), 1, "number of messages created");
        let expected = vec![0x03, 0x01, 0x12, 0xe9, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(msgs[0].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[0][..8], "checking message");
        Ok(())
    }

    #[test]
    fn calculator() -> anyhow::Result<()> {
        // 03 01 12 e9 01 000000...
        let kbd = Keyboard8890::new(None, 0)?;
        let msgs = kbd.map_key("calculator".to_string(), 1)?;
        println!("{:02x?}", msgs);
        assert_eq!(msgs.len(), 1, "number of messages created");
        let expected = vec![0x03, 0x01, 0x12, 0x92, 0x01, 0x00, 0x00, 0x00];
        assert_eq!(msgs[0].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[0][..8], "checking message");
        Ok(())
    }

    #[test]
    fn back() -> anyhow::Result<()> {
        // 03 01 12 24 02 000000...
        let kbd = Keyboard8890::new(None, 0)?;
        let msgs = kbd.map_key("webpageback".to_string(), 1)?;
        println!("{:02x?}", msgs);
        assert_eq!(msgs.len(), 1, "number of messages created");
        let expected = vec![0x03, 0x01, 0x12, 0x24, 0x02, 0x00, 0x00, 0x00];
        assert_eq!(msgs[0].len(), consts::PACKET_SIZE, "checking msg size");
        assert_eq!(&expected, &msgs[0][..8], "checking message");
        Ok(())
    }
}
