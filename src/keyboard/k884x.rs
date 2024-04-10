use anyhow::{ensure, Result};
use log::debug;
use rusb::{Context, DeviceHandle};

use crate::consts;

use super::{Key, Keyboard, Macro, MouseAction, MouseEvent};

pub struct Keyboard884x {
    handle: DeviceHandle<Context>,
    out_endpoint: u8,
    in_endpoint: u8,
}

impl Keyboard for Keyboard884x {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()> {
        ensure!(layer <= 15, "invalid layer index");

        debug!("bind {} on layer {} to {}", key, layer, expansion);

        let mut msg = vec![
            0x03,
            0xfd,
            key.to_key_id_16()?,
            layer + 1,
            expansion.kind(),
            0,
            0,
            0,
            0,
            0,
        ];

        match expansion {
            Macro::Keyboard(presses) => {
                ensure!(
                    presses.len() <= consts::MAX_KEY_PRESSES,
                    "macro sequence is too long"
                );
                // For whatever reason empty key is added before others.
                let iter = presses.iter().map(|accord| {
                    (
                        accord.modifiers.as_u8(),
                        accord.code.map_or(0, |c| c.value()),
                    )
                });

                msg.extend_from_slice(&[presses.len() as u8]);
                for (modifiers, code) in iter {
                    msg.extend_from_slice(&[modifiers, code]);
                }
            }
            Macro::Media(code) => {
                let [low, high] = (*code as u16).to_le_bytes();
                msg.extend_from_slice(&[0, low, high, 0, 0, 0, 0]);
            }
            Macro::Mouse(MouseEvent(MouseAction::Click(buttons), _)) => {
                ensure!(!buttons.is_empty(), "buttons must be given for click macro");
                msg.extend_from_slice(&[0x01, 0, buttons.as_u8()]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelUp, modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, 0x1]);
            }
            Macro::Mouse(MouseEvent(MouseAction::WheelDown, modifier)) => {
                msg.extend_from_slice(&[0x03, modifier.map_or(0, |m| m as u8), 0, 0, 0, 0xff]);
            }
        };

        self.send(&msg)?;

        Ok(())
    }

    fn set_led(&mut self, _n: u8) -> Result<()> {
        unimplemented!(
            "If you have a device which supports backlight LEDs, please let us know so \
                        we can help you reverse-engineer it."
        )
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
