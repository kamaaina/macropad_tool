use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

#[derive(Debug, EnumString, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[strum(ascii_case_insensitive)]
pub enum Orientation {
    Normal,
    UpsideDown,
    Clockwise,
    CounterClockwise,
}

fn transpose<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    assert!(!v.is_empty());
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}

/// Returns the key numbers transposed clockwise
///
/// #Arguments
/// `keys` - the key number maxtrix
///
pub fn get_keys_clockwise(keys: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut data: Vec<Vec<u8>> = vec![];
    for i in keys.iter().rev() {
        data.push(i.to_vec());
    }
    transpose(data)
}

/// Returns the key numbers transposed counter clockwise
///
/// #Arguments
/// `keys` - the key number maxtrix
///
pub fn get_keys_counter_clockwise(keys: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut data: Vec<Vec<u8>> = vec![];
    for mut i in keys {
        i.reverse();
        data.push(i);
    }
    transpose(data)
}

/// Returns the key numbers flipped upside down
///
/// #Arguments
/// `keys` - the key number maxtrix
///
pub fn get_keys_upsidedown(keys: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut data: Vec<Vec<u8>> = vec![];
    for i in keys.iter().rev() {
        let mut tmp = i.clone();
        tmp.reverse();
        data.push(tmp.to_vec());
    }
    data
}

#[cfg(test)]
#[test]
fn test_clockwise() {
    let mut keys = Vec::new();
    let mut a = Vec::new();
    for i in 1..=3 {
        a.push(i);
    }
    keys.push(a);

    let mut a = Vec::new();
    for i in 4..=6 {
        a.push(i);
    }
    keys.push(a);

    let transposed = get_keys_clockwise(keys.clone());
    assert_eq!(transposed[0], [4, 1]);
    assert_eq!(transposed[1], [5, 2]);
    assert_eq!(transposed[2], [6, 3]);
}

#[test]
fn test_counter_clockwise() {
    let mut keys = Vec::new();
    let mut a = Vec::new();
    for i in 1..=3 {
        a.push(i);
    }
    keys.push(a);

    let mut a = Vec::new();
    for i in 4..=6 {
        a.push(i);
    }
    keys.push(a);

    let transposed = get_keys_counter_clockwise(keys.clone());
    assert_eq!(transposed[0], [3, 6]);
    assert_eq!(transposed[1], [2, 5]);
    assert_eq!(transposed[2], [1, 4]);
}

#[test]
fn test_upside_down() {
    let mut keys = Vec::new();
    let mut a = Vec::new();
    for i in 1..=3 {
        a.push(i);
    }
    keys.push(a);

    let mut a = Vec::new();
    for i in 4..=6 {
        a.push(i);
    }
    keys.push(a);

    let usd = get_keys_upsidedown(keys.clone());
    assert_eq!(usd[0], [6, 5, 4]);
    assert_eq!(usd[1], [3, 2, 1]);
}
