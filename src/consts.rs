use std::time::Duration;

pub const VENDOR_ID: u16 = 0x1189;
pub const PRODUCT_IDS: [u16; 3] = [0x8840, 0x8842, 0x8890];

/// Timeout for reading from USB
///
pub const TIMEOUT: Duration = Duration::from_millis(100);

/// Number of layers on the macropad. Depending on the model,
/// some layers are no accessible
///
pub const NUM_LAYERS: u8 = 3;

/// Read buffer size (in bytes)
///
pub const READ_BUF_SIZE: u8 = 72;

/// Maximum number of keys that can be assigned to a KeyChord
///
pub const MAX_KEYCHORD: usize = 17;

/// Maximum delay for a keypress
///
pub const MAX_DELAY: u16 = 6000;
