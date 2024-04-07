pub(crate) mod k884x;
pub(crate) mod k8880;

use crate::parse;

use std::{fmt::Display, str::FromStr, time::Duration};

use anyhow::{anyhow, ensure, Result};
use enumset::{EnumSet, EnumSetType};
use log::debug;
use rusb::{Context, DeviceHandle, Error::Timeout};
use serde_with::DeserializeFromStr;
use strum_macros::{Display, EnumIter, EnumMessage, EnumString};

use itertools::Itertools as _;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);

pub trait Keyboard {
    fn bind_key(&mut self, layer: u8, key: Key, expansion: &Macro) -> Result<()>;
    fn set_led(&mut self, n: u8) -> Result<()>;

    fn get_handle(&self) -> &DeviceHandle<Context>;
    fn get_out_endpoint(&self) -> u8;
    fn get_in_endpoint(&self) -> u8;

    fn send(&mut self, msg: &[u8]) -> Result<()> {
        let mut buf = [0; 65];
        buf.iter_mut().zip(msg.iter()).for_each(|(dst, src)| {
            *dst = *src;
        });

        debug!("send: {:02x?}", buf);
        let written =
            self.get_handle()
                .write_interrupt(self.get_out_endpoint(), &buf, DEFAULT_TIMEOUT)?;
        ensure!(written == buf.len(), "not all data written");
        Ok(())
    }

    fn recieve(&mut self, buf: &mut [u8]) -> Result<usize> {
        let read = self
            .get_handle()
            .read_interrupt(self.get_in_endpoint(), buf, DEFAULT_TIMEOUT);

        let mut bytes_read = 0;
        if read.is_err() {
            let e = read.err().unwrap();
            match e {
                Timeout => {
                    debug!("timeout on read");
                }

                _ => {
                    eprintln!("error reading interrupt - {}", e);
                }
            };
        } else {
            bytes_read = read.unwrap();
        }

        debug!("bytes read: {bytes_read}");
        debug!("data: {:02x?}", buf);
        debug!("----------------------------------------------------");

        Ok(bytes_read)
    }
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

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Button(u8),
    #[allow(unused)]
    Knob(u8, KnobAction),
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Button(n) => write!(f, "button {}", n),
            Self::Knob(n, action) => write!(f, "knob {} {}", n, action),
        }
    }
}

impl Key {
    fn to_key_id_12(self) -> Result<u8> {
        match self {
            Key::Button(n) if n >= 12 => Err(anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= 3 => Err(anyhow!("invalid knob index")),
            Key::Knob(n, action) => Ok(13 + 3 * n + (action as u8)),
        }
    }

    fn to_key_id_16(self) -> Result<u8> {
        match self {
            Key::Button(n) if n >= 16 => Err(anyhow!("invalid key index")),
            Key::Button(n) => Ok(n + 1),
            Key::Knob(n, _) if n >= 3 => Err(anyhow!("invalid knob index")),
            Key::Knob(n, action) => Ok(16 + 3 * n + (action as u8)),
        }
    }
}

#[derive(Debug, EnumSetType, EnumString, EnumIter, EnumMessage, Display)]
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

pub type Modifiers = EnumSet<Modifier>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, EnumIter, EnumMessage, Display)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    WellKnown(WellKnownCode),
    Custom(u8),
}

impl Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Code::WellKnown(code) => write!(f, "{}", code),
            Code::Custom(code) => write!(f, "<{}>", code),
        }
    }
}

impl From<WellKnownCode> for Code {
    fn from(code: WellKnownCode) -> Self {
        Self::WellKnown(code)
    }
}

impl FromStr for Code {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parse::from_str(parse::code, s)
    }
}

impl Code {
    pub fn value(self) -> u8 {
        match self {
            Self::WellKnown(code) => code as u8,
            Self::Custom(code) => code,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, EnumIter, Display)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, DeserializeFromStr)]
pub struct Accord {
    pub modifiers: Modifiers,
    pub code: Option<Code>,
}

impl Accord {
    pub fn new<M>(modifiers: M, code: Option<Code>) -> Self
    where
        M: Into<Modifiers>,
    {
        Self {
            modifiers: modifiers.into(),
            code,
        }
    }
}

impl From<(Modifiers, Option<Code>)> for Accord {
    fn from((modifiers, code): (Modifiers, Option<Code>)) -> Self {
        Self { modifiers, code }
    }
}

impl FromStr for Accord {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parse::from_str(parse::accord, s)
    }
}

impl Display for Accord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.modifiers.iter().format("-"))?;
        if let Some(code) = self.code {
            if !self.modifiers.is_empty() {
                write!(f, "-")?;
            }
            write!(f, "{}", code)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[strum(ascii_case_insensitive)]
#[repr(u8)]
pub enum MouseModifier {
    Ctrl = 0x01,
    Shift = 0x02,
    Alt = 0x04,
}

#[derive(Debug, EnumSetType, EnumIter, Display)]
pub enum MouseButton {
    #[strum(serialize = "click")]
    Left,
    #[strum(serialize = "rclick")]
    Right,
    #[strum(serialize = "mclick")]
    Middle,
}

pub type MouseButtons = EnumSet<MouseButton>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent(pub MouseAction, pub Option<MouseModifier>);

impl Display for MouseEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(action, modifier) = self;
        if let Some(modifier) = modifier {
            write!(f, "{}-", modifier)?;
        }
        write!(f, "{}", action)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, DeserializeFromStr)]
pub enum Macro {
    Keyboard(Vec<Accord>),
    #[allow(unused)]
    Media(MediaCode),
    #[allow(unused)]
    Mouse(MouseEvent),
}

impl Macro {
    fn kind(&self) -> u8 {
        match self {
            Macro::Keyboard(_) => 1,
            Macro::Media(_) => 2,
            Macro::Mouse(_) => 3,
        }
    }
}

impl FromStr for Macro {
    type Err = nom::error::Error<String>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parse::from_str(parse::r#macro, s)
    }
}

impl Display for Macro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Macro::Keyboard(accords) => {
                write!(f, "{}", accords.iter().format(","))
            }
            Macro::Media(code) => {
                write!(f, "{}", code)
            }
            Macro::Mouse(event) => {
                write!(f, "{}", event)
            }
        }
    }
}
