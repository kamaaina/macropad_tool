pub(crate) mod k884x;
pub(crate) mod k8890;

use crate::{config, config::Orientation, consts, mapping::Macropad};

use std::fmt::Display;

use anyhow::{ensure, Result};
use enumset::{EnumSet, EnumSetType};
use log::debug;
use num_derive::{FromPrimitive, ToPrimitive};
use rusb::{Context, DeviceHandle, Error::Timeout};
use strum_macros::{Display, EnumIter, EnumMessage, EnumString};

use itertools::Itertools as _;

pub trait Messages {
    /// Returns the message to the macropad to get its configuration
    ///
    /// #Arguments
    /// `keys` - number of keys on device
    /// `encoders` - number of endoders on device
    /// `layer` - layer to read
    ///
    fn read_config(&self, keys: u8, encoders: u8, layer: u8) -> Vec<u8>;

    /// Returns the message to get the device type
    ///
    fn device_type(&self) -> Vec<u8>;

    /// Returns the message to program the LEDs on the macropad based on the
    /// specified `mode` and `color`
    ///
    /// #Arguments
    /// `mode` - preset mode of the LED
    /// `layer` - layer to program
    /// `color` - the color to use for the mode
    ///
    fn program_led(&self, mode: u8, layer: u8, color: LedColor) -> Vec<u8>;

    /// Returns the "end of programming" message for the device. This message
    /// effectively tell the device to 'save its configuration' so when it is
    /// unplugged, it retains its settings
    ///
    fn end_program(&self) -> Vec<u8>;
}

pub trait Configuration {
    /// Returns the Macropad with its configuration settings for the specified layer
    ///
    /// #Arguments
    /// `layer` - layer to read configuration for
    ///
    fn read_macropad_config(&mut self, layer: &u8) -> Result<Macropad>;

    /// Returns the layout button configuration for the specified orientation
    ///
    /// #Arguments
    /// `orientation` - orientation of the macropad
    /// `rows` - number of rows
    /// `cols` - number of columns
    ///
    fn get_layout(&self, orientation: Orientation, rows: u8, cols: u8) -> Result<Vec<Vec<u8>>> {
        // normalize layout to "normal" orientation
        let default_layout = if orientation == Orientation::Clockwise
            || orientation == Orientation::CounterClockwise
        {
            // transpose
            self.default_key_numbers(cols, rows)
        } else {
            self.default_key_numbers(rows, cols)
        };
        debug!("default_layout: {default_layout:?}");

        let layout = match orientation {
            Orientation::Clockwise => config::get_keys_clockwise(default_layout),
            Orientation::CounterClockwise => config::get_keys_counter_clockwise(default_layout),
            Orientation::UpsideDown => config::get_keys_upsidedown(default_layout),
            Orientation::Normal => default_layout,
        };

        Ok(layout)
    }

    /// Returns the default 'normal' orientation button numbers for programming
    ///
    /// #Arguments
    /// `rows` - number of rows
    /// `cols` - number of columns
    ///
    fn default_key_numbers(&self, rows: u8, cols: u8) -> Vec<Vec<u8>> {
        let mut layout: Vec<Vec<u8>> = Vec::new();
        let mut idx = 1u8;
        for _i in 0..rows {
            let mut tmp = Vec::new();
            for _j in 0..cols {
                tmp.push(idx);
                idx += 1;
            }
            layout.push(tmp);
        }
        layout
    }
}

pub trait Keyboard: Messages + Configuration {
    /// Programs the macropad based on the specified `Macropad`
    ///
    /// #Arguments
    /// `macropad` - configuration to be programmed
    ///
    fn program(&mut self, macropad: &Macropad) -> Result<()>;

    /// Programs the LEDs on the macropad
    ///
    /// #Arguments
    /// `mode` - preset mode of the LED
    /// `layer` - layer to program
    /// `color` - the color to use for the mode
    ///
    fn set_led(&mut self, mode: u8, layer: u8, color: LedColor) -> Result<()>;

    /// Returns the handle of the device
    ///
    fn get_handle(&self) -> &DeviceHandle<Context>;

    /// Returns the out endpoint of the device (write)
    ///
    fn get_out_endpoint(&self) -> u8;

    /// Returns the in endpoint of the device (read)
    ///
    fn get_in_endpoint(&self) -> u8;

    /// Sends the specified `msg` over usb to the out endpoint as an
    /// USB Interrupt out message. Error is through if not all bytes
    /// of the message could be sent
    ///
    /// #Arguments
    /// `msg` - message to be sent
    ///
    fn send(&mut self, msg: &[u8]) -> Result<()> {
        let written = self.get_handle().write_interrupt(
            self.get_out_endpoint(),
            msg,
            consts::DEFAULT_TIMEOUT,
        )?;
        ensure!(written == msg.len(), "not all data written");
        debug!("msg: {msg:02x?}");
        debug!("--------------------------------------------------");
        Ok(())
    }

    /// Reads data from macropad and stores it in buf
    ///
    /// #Arguments
    /// `buf` - buffer to store the data that is read
    ///
    fn recieve(&mut self, buf: &mut [u8]) -> Result<usize> {
        let read =
            self.get_handle()
                .read_interrupt(self.get_in_endpoint(), buf, consts::DEFAULT_TIMEOUT);

        let mut bytes_read = 0;
        if let Err(e) = read {
            match e {
                Timeout => {
                    debug!("timeout on read");
                    return Ok(0);
                }

                _ => {
                    eprintln!("error reading interrupt - {e}");
                }
            };
        } else {
            bytes_read = read.unwrap();
        }

        debug!("bytes read: {bytes_read}");
        debug!("data: {buf:02x?}");

        Ok(bytes_read)
    }
}

#[derive(Debug, Default, ToPrimitive, Clone, Copy, Display, clap::ValueEnum)]
pub enum LedColor {
    Red = 0x10,
    Orange = 0x20,
    Yellow = 0x30,
    Green = 0x40,
    #[default]
    Cyan = 0x50,
    Blue = 0x60,
    Purple = 0x70,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, Display)]
#[repr(u8)]
pub enum KnobAction {
    #[strum(serialize = "ccw")]
    RotateCCW,
    #[strum(serialize = "press")]
    Press,
    #[strum(serialize = "cw")]
    RotateCW,
}

#[derive(
    Debug, ToPrimitive, FromPrimitive, EnumSetType, EnumString, EnumIter, EnumMessage, Display,
)]
#[strum(ascii_case_insensitive)]
pub enum Modifier {
    #[strum(serialize = "ctrl")]
    Ctrl,
    #[strum(serialize = "shift")]
    Shift,
    #[strum(serialize = "alt", serialize = "opt")]
    Alt,
    #[strum(serialize = "win", serialize = "cmd")]
    Win,
    #[strum(serialize = "rctrl")]
    RightCtrl,
    #[strum(serialize = "rshift")]
    RightShift,
    #[strum(serialize = "ralt", serialize = "ropt")]
    RightAlt,
    #[strum(serialize = "rwin", serialize = "rcmd")]
    RightWin,
}

#[derive(
    Debug,
    FromPrimitive,
    ToPrimitive,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumString,
    EnumIter,
    EnumMessage,
    Display,
)]
#[repr(u16)]
#[strum(serialize_all = "lowercase")]
#[strum(ascii_case_insensitive)]
pub enum MediaCode {
    Next = 0xb5,
    #[strum(serialize = "previous", serialize = "prev")]
    Previous = 0xb6,
    Stop = 0xb7,
    Play = 0xcd,
    Mute = 0xe2,
    VolumeUp = 0xe9,
    VolumeDown = 0xea,
    Favorites = 0x182,
    Calculator = 0x192,
    ScreenLock = 0x19e,
    ScreenBrightnessUp = 0x6f,
    ScreenBrightnessDown = 0x70,
    WebPageHome = 0x0223,
    WebPageBack = 0x0224,
    WebPageForward = 0x0225,
}

#[derive(
    Debug, ToPrimitive, FromPrimitive, Clone, Copy, PartialEq, Eq, EnumString, EnumIter, Display,
)]
#[repr(u8)]
#[strum(ascii_case_insensitive)]
#[strum(serialize_all = "lowercase")]
pub enum WellKnownCode {
    A = 0x04,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    #[strum(serialize = "1")]
    N1,
    #[strum(serialize = "2")]
    N2,
    #[strum(serialize = "3")]
    N3,
    #[strum(serialize = "4")]
    N4,
    #[strum(serialize = "5")]
    N5,
    #[strum(serialize = "6")]
    N6,
    #[strum(serialize = "7")]
    N7,
    #[strum(serialize = "8")]
    N8,
    #[strum(serialize = "9")]
    N9,
    #[strum(serialize = "0")]
    N0,
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    NonUSHash,
    Semicolon,
    Quote,
    Grave,
    Comma,
    Dot,
    Slash,
    CapsLock,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    Delete,
    End,
    PageDown,
    Right,
    Left,
    Down,
    Up,
    NumLock,
    NumPadSlash,
    NumPadAsterisk,
    NumPadMinus,
    NumPadPlus,
    NumPadEnter,
    NumPad1,
    NumPad2,
    NumPad3,
    NumPad4,
    NumPad5,
    NumPad6,
    NumPad7,
    NumPad8,
    NumPad9,
    NumPad0,
    NumPadDot,
    NonUSBackslash,
    Application,
    Power,
    NumPadEqual,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
}

#[allow(dead_code)] // TODO: implement
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[strum(ascii_case_insensitive)]
#[repr(u8)]
pub enum MouseModifier {
    Ctrl = 0x01,
    Shift = 0x02,
    Alt = 0x04,
}

#[derive(ToPrimitive, EnumString, Debug, EnumSetType, EnumIter, Display)]
pub enum MouseButton {
    #[strum(serialize = "click")]
    Left,
    #[strum(serialize = "rclick")]
    Right,
    #[strum(serialize = "mclick")]
    Middle,
}

pub type MouseButtons = EnumSet<MouseButton>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum MouseAction {
    Click(MouseButtons),
    WheelUp,
    WheelDown,
}

impl Display for MouseAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MouseAction::Click(buttons) => {
                write!(f, "{}", buttons.iter().format("+"))?;
            }
            MouseAction::WheelUp => {
                write!(f, "wheelup")?;
            }
            MouseAction::WheelDown => {
                write!(f, "wheeldown")?;
            }
        }
        Ok(())
    }
}

#[allow(dead_code)] // TODO: implement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent(pub MouseAction, pub Option<MouseModifier>);

impl Display for MouseEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(action, modifier) = self;
        if let Some(modifier) = modifier {
            write!(f, "{modifier}-")?;
        }
        write!(f, "{action}")?;
        Ok(())
    }
}
