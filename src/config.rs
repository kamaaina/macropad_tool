use serde::Deserialize;
use strum_macros::EnumString;

#[derive(Debug, EnumString, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Normal,
    UpsideDown,
    Clockwise,
    CounterClockwise,
}
