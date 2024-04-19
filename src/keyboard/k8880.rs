use anyhow::Result;
use rusb::{Context, DeviceHandle};

use super::{Keyboard, LedColor};

pub struct Keyboard8880 {
    handle: DeviceHandle<Context>,
    out_endpoint: u8,
    in_endpoint: u8,
}

impl Keyboard for Keyboard8880 {
    fn map_key(&mut self, _layer: u8, _key_num: u8, _key: String) -> Result<()> {
        unimplemented!("i do not have this macropad to test on");
    }

    fn set_led(&mut self, n: u8, _color: LedColor) -> Result<()> {
        self.send(&[0xa1, 0x01, 0, 0, 0, 0, 0, 0])?;
        self.send(&[0xb0, 0x18, n, 0, 0, 0, 0, 0])?;
        self.send(&[0xaa, 0xa1, 0, 0, 0, 0, 0, 0])?;
        Ok(())
    }

    fn get_handle(&self) -> &DeviceHandle<Context> {
        &self.handle
    }

    fn get_out_endpoint(&self) -> u8 {
        self.out_endpoint
    }

    fn get_in_endpoint(&self) -> u8 {
        self.in_endpoint
    }
}

impl Keyboard8880 {
    pub fn new(handle: DeviceHandle<Context>, out_endpoint: u8, in_endpoint: u8) -> Result<Self> {
        let keyboard = Self {
            handle,
            out_endpoint,
            in_endpoint,
        };

        Ok(keyboard)
    }
}
