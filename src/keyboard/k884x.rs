use anyhow::Result;
use log::debug;
use rusb::{Context, DeviceHandle};

use crate::messages::{self, Messages};

use super::{Keyboard, LedColor};

pub struct Keyboard884x {
    handle: DeviceHandle<Context>,
    out_endpoint: u8,
    in_endpoint: u8,
}

impl Keyboard for Keyboard884x {
    fn map_key(&mut self, layer: u8, key_num: u8, key_chord: String) -> Result<()> {
        debug!("layer: {layer} key_num: {key_num} key_chord: {key_chord}");
        let msg = Messages::build_key_msg(key_chord, layer, key_num, 0)?;
        self.send(&msg)?;
        Ok(())
    }

    fn set_led(&mut self, n: u8, color: LedColor) -> Result<()> {
        self.send(&messages::Messages::program_led(n, color))?;
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

impl Keyboard884x {
    pub fn new(handle: DeviceHandle<Context>, out_endpoint: u8, in_endpoint: u8) -> Result<Self> {
        let keyboard = Self {
            handle,
            out_endpoint,
            in_endpoint,
        };

        Ok(keyboard)
    }
}
