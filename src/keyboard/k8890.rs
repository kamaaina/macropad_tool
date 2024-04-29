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

#[derive(PartialEq)]
enum MsgType {
    KeyBeginProgram,
    KeyProgram,
}

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
    fn read_supported(&self) -> bool {
        false
    }

    fn read_config(&self, _keys: u8, _encoders: u8, _layer: u8) -> Vec<u8> {
        unimplemented!("reading configuration from this macropad is not supported");
    }

    fn device_type(&self) -> Vec<u8> {
        unimplemented!("reading device type is not supported");
    }

    fn program_led(&self, mode: u8, _color: LedColor) -> Vec<u8> {
        let mut msg = vec![0x03, 0xb0, 0x18, mode];
        msg.extend_from_slice(&[0; 61]);
        msg
    }

    fn end_program(&self) -> Vec<u8> {
        let last_byte = if self.led_programmed { 0xa1 } else { 0xaa };
        let mut msg = vec![0x03, 0xaa, last_byte];
        msg.extend_from_slice(&[0; 62]);
        msg
    }
}

impl Keyboard for Keyboard8890 {
    fn program(&mut self, macropad: &Macropad) -> Result<()> {
        debug!("programming keyboard");
        self.send(&self.begin_programming())?;
        // my device only has 1 layer. one row with 4 keys, but i have to believe that this same chip
        // powers different configurations so lets try to program the macropad according to the
        // configuration file rather than hardcoding it to only 4 keys
        for (i, layer) in macropad.layers.iter().enumerate() {
            let mut j = 1;
            for row in &layer.buttons {
                for btn in row {
                    debug!("program layer: {} key: 0x{:02x} to: {btn}", i + 1, j);
                    let keys: Vec<_> = btn.split(',').collect();
                    if keys.len() > consts::MAX_KEY_PRESSES_8890 {
                        return Err(anyhow!(
                            "maximum key presses for this macropad is {}",
                            consts::MAX_KEY_PRESSES_8890
                        ));
                    }
                    let msg = self.build_key_msg(
                        btn.to_string(),
                        keys.len().try_into()?,
                        j,
                        MsgType::KeyBeginProgram,
                        0,
                    )?;
                    if msg[2] != 0x13 {
                        // for mouse related key presses, there is just one message
                        // per key press when programming key
                        //j += 1;
                        //continue;
                        self.send(&msg)?;
                    }
                    for (idx, key) in keys.iter().enumerate() {
                        self.send(&self.build_key_msg(
                            key.to_string(),
                            keys.len().try_into()?,
                            j,
                            MsgType::KeyProgram,
                            (idx + 1).try_into()?,
                        )?)?;
                    }
                    j += 1;
                }
            }
        }
        self.send(&self.end_program())?;
        debug!("DONE - programming keyboard");
        Ok(())
    }

    fn set_led(&mut self, mode: u8, _color: LedColor) -> Result<()> {
        if mode > 2 {
            return Err(anyhow!("macropad supports modes 0, 1, and 2 only"));
        }
        self.led_programmed = true;
        self.send(&self.begin_programming())?;
        self.send(&self.program_led(mode, LedColor::Red))?;
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

    pub fn begin_programming(&self) -> Vec<u8> {
        let mut msg = vec![0x03, 0xa1, 0x01];
        msg.extend_from_slice(&[0; 62]);
        msg
    }

    fn build_key_msg(
        &self,
        key_chord: String,
        num_keys_to_pgrm: u8,
        key_pos: u8,
        msg_type: MsgType,
        cur_pgrm_key: u8,
    ) -> Result<Vec<u8>> {
        let mut msg = vec![0x03, key_pos, 0x11, 0x00, 0x00];

        // it seems this devices needs at least 2 messages to program one key so need
        // to differentiate if we are building first or second message
        let kc: Vec<_> = key_chord.split('-').collect();
        let mut remaining = 60;
        msg[3] = num_keys_to_pgrm;
        if msg_type == MsgType::KeyProgram {
            debug!("=== KeyProgram ===");
            let mut mouse_action = 0u8;
            let mut mouse_click;
            let kc: Vec<_> = key_chord.split('-').collect();
            msg[4] += cur_pgrm_key;
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
                } else if let Ok(a) = MouseButton::from_str(key) {
                    mouse_click =
                        2u32.pow(<MouseButton as ToPrimitive>::to_u8(&a).unwrap().into()) as u8;
                    msg[2] = 0x13;
                    msg[3] = mouse_click;
                } else if let Ok(a) = MouseAction::from_str(key) {
                    match a {
                        MouseAction::WheelUp => mouse_action = 0x01,
                        MouseAction::WheelDown => mouse_action = 0xff,
                        _ => (),
                    }
                    msg[2] = 0x13;
                    msg[6] = mouse_action;
                }
            }
            msg.extend_from_slice(&[m_c, wkk]);
            remaining -= 2;
        } else {
            debug!("=== KeyBeginProgram ===");
            for key in kc {
                debug!("=> {key}");
                if let Ok(_a) = MouseButton::from_str(key) {
                    msg[2] = 0x13;
                } else if let Ok(_a) = MouseAction::from_str(key) {
                    msg[2] = 0x13;
                }
            }
            msg.extend_from_slice(&[0x01]);
            remaining -= 1;
        }
        msg.extend_from_slice(&vec![0; remaining]);

        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use crate::keyboard::{
        k8890::{Keyboard8890, MsgType},
        LedColor, Messages,
    };

    #[test]
    fn ctrl_a_ctrl_s() -> anyhow::Result<()> {
        let kbd = Keyboard8890::new(None, 0)?;
        let msg = kbd.build_key_msg(
            "ctrl-a,ctrl-s".to_string(),
            1u8,
            1u8,
            MsgType::KeyBeginProgram,
            0,
        )?;
        let expected = vec![0x03, 0x01, 0x11, 0x01, 0x00, 0x01];
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(&expected, &msg[..6], "checking message");

        let msg = kbd.build_key_msg("ctrl-a".to_string(), 1u8, 1u8, MsgType::KeyProgram, 1)?;
        let expected = vec![0x03, 0x01, 0x11, 0x01, 0x01, 0x01, 0x04];
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(&expected, &msg[..7], "checking message");

        let msg = kbd.build_key_msg("ctrl-s".to_string(), 1u8, 1u8, MsgType::KeyProgram, 1)?;
        let expected = vec![0x03, 0x01, 0x11, 0x01, 0x01, 0x01, 0x16];
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(&expected, &msg[..7], "checking message");
        Ok(())
    }

    #[test]
    fn a_key() -> anyhow::Result<()> {
        let kbd = Keyboard8890::new(None, 0)?;
        let msg = kbd.build_key_msg("a".to_string(), 1u8, 1u8, MsgType::KeyBeginProgram, 0)?;
        let expected = vec![0x03, 0x01, 0x11, 0x01, 0x00, 0x01];
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(&expected, &msg[..6], "checking message");
        let msg = kbd.build_key_msg("a".to_string(), 1u8, 1u8, MsgType::KeyProgram, 1)?;

        let expected = vec![0x03, 0x01, 0x11, 0x01, 0x01, 0x00, 0x04];
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(&expected, &msg[..7], "checking message");
        Ok(())
    }

    #[test]
    fn led_mode2() -> anyhow::Result<()> {
        let mut kbd = Keyboard8890::new(None, 0)?;
        kbd.led_programmed = true;
        let msg = kbd.program_led(2, LedColor::Red);
        println!("{:02x?}", msg);
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[1], 0xb0, "checking first byte of led programming");
        assert_eq!(msg[2], 0x18, "checking second byte of led programming");
        assert_eq!(msg[3], 0x02, "checking led mode");
        let msg = kbd.end_program();
        assert_eq!(msg.len(), 65, "checking msg size");
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
        assert_eq!(msg.len(), 65, "checking msg size");
        assert_eq!(msg[0], 0x03, "checking first byte of end programming");
        assert_eq!(msg[1], 0xaa, "checking second byte of end programming");
        assert_eq!(msg[2], 0xaa, "checking third byte of end programming");
        Ok(())
    }
}
