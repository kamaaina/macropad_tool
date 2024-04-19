use serde::Deserialize;
use strum_macros::EnumString;

use crate::keyboard::Macro;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub orientation: Orientation,
    pub rows: u8,
    pub columns: u8,
    pub knobs: u8,
    pub layers: Vec<Layer>,
}

#[derive(Debug, EnumString, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Normal,
    UpsideDown,
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Deserialize)]
pub struct Layer {
    pub buttons: Vec<Vec<Option<Macro>>>,
    pub knobs: Vec<Knob>,
}

#[derive(Debug, Deserialize)]
pub struct Knob {
    pub ccw: Option<Macro>,
    pub press: Option<Macro>,
    pub cw: Option<Macro>,
}
