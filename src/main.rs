mod config;
mod consts;
mod decoder;
mod keyboard;
mod mapping;
mod messages;
mod options;
mod parse;

use crate::consts::PRODUCT_IDS;
use crate::decoder::KeyMapping;
use crate::keyboard::{
    k884x, k8880, Keyboard, MediaCode, Modifier, MouseAction, MouseButton, WellKnownCode,
};
use crate::mapping::Macropad;
use crate::messages::Messages;
use crate::options::Options;
use crate::options::{Command, LedCommand};

use anyhow::{anyhow, ensure, Result};
use indoc::indoc;
use itertools::Itertools;
use log::{debug, info};
use mapping::Mapping;
use rusb::{Context, Device, DeviceDescriptor, Direction, TransferType};

use anyhow::Context as _;
use clap::Parser as _;
use rusb::UsbContext as _;
use strum::EnumMessage as _;
use strum::IntoEnumIterator as _;

fn main() -> Result<()> {
    env_logger::init();
    let options = Options::parse();

    match &options.command {
        Command::ShowKeys => {
            println!("Modifiers: ");
            for m in Modifier::iter() {
                println!(" - {}", m.get_serializations().iter().join(" / "));
            }

            println!();
            println!("Keys:");
            for c in WellKnownCode::iter() {
                println!(" - {c}");
            }

            println!();
            println!("Custom key syntax (use decimal code): <110>");

            println!();
            println!("Media keys:");
            for c in MediaCode::iter() {
                println!(" - {}", c.get_serializations().iter().join(" / "));
            }

            println!();
            println!("Mouse actions:");
            println!(" - {}", MouseAction::WheelDown);
            println!(" - {}", MouseAction::WheelUp);
            for b in MouseButton::iter() {
                println!(" - {b}");
            }
        }

        Command::Validate { config_file } => {
            // load and validate mapping
            Mapping::validate(config_file).context("validating configuration file")?;
            println!("config is valid 👌")
        }

        Command::Program { config_file } => {
            // load and validate mapping
            Mapping::validate(config_file)?;
            let config = Mapping::read(config_file);
            let mut keyboard = open_keyboard(&options)?;

            // ensure the config we have matches the connected device we want to program
            let mut buf = vec![0; consts::READ_BUF_SIZE.into()];

            // get the type of device
            let _ = keyboard.send(&messages::Messages::device_type());
            let _ = keyboard.recieve(&mut buf);
            let device_info = decoder::Decoder::get_device_info(&buf);
            ensure!(
                device_info.num_keys == (config.device.rows * config.device.cols)
                    && device_info.num_encoders == config.device.knobs,
                "Configuration file and macropad mismatch.\nLooks like you are trying to program a different macropad.\nDid you select the right configuration file?\n"
            );

            for (i, layer) in config.layers.iter().enumerate() {
                let lyr = (i + 1) as u8;
                let mut j = 1;
                for row in &layer.buttons {
                    for btn in row {
                        debug!("program layer: {} key: 0x{:02x} to: {btn}", i + 1, j);
                        keyboard.map_key(lyr, j, btn.to_string())?;
                        j += 1;
                    }
                }

                // TODO: test 9x3 to see if the 3 knobs are top to bottom with key number
                j = 0x10;
                for knob in &layer.knobs {
                    debug!("layer: {} key: 0x{:02x} knob cw {}", i + 1, j, knob.cw);
                    keyboard.map_key(lyr, j, knob.cw.clone())?;
                    j += 1;

                    debug!(
                        "layer: {} key: 0x{:02x} knob press {}",
                        i + 1,
                        j,
                        knob.press
                    );
                    keyboard.map_key(lyr, j, knob.press.clone())?;
                    j += 1;

                    debug!("layer: {} key: 0x{:02x} knob ccw {}", i + 1, j, knob.ccw);
                    keyboard.map_key(lyr, j, knob.ccw.clone())?;
                    j += 1;
                }
            }
            let _ = keyboard.send(&Messages::end_program());
            println!("デバイスのプログラミングが完了しました");
        }

        Command::Led(LedCommand { index, led_color }) => {
            let mut keyboard = open_keyboard(&options)?;
            keyboard.set_led(*index, *led_color)?;
            let _ = keyboard.send(&Messages::end_program());
        }

        Command::Read { layer } => {
            debug!("dev options: {:?}", options.devel_options);
            let mut buf = vec![0; consts::READ_BUF_SIZE.into()];
            let mut keyboard = open_keyboard(&options)?;

            // get the type of device
            let _ = keyboard.send(&messages::Messages::device_type());
            let _ = keyboard.recieve(&mut buf);
            let device_info = decoder::Decoder::get_device_info(&buf);
            info!(
                "OUT: 0x{:02x} IN: 0x{:02x}",
                keyboard.get_out_endpoint(),
                keyboard.get_in_endpoint()
            );
            debug!(
                "number of keys: {} number of rotary encoders: {}",
                device_info.num_keys, device_info.num_encoders
            );

            // send message to get keys and process later so we don't slow the usb traffic
            // not sure if that would be an issue as i don't know the usb protocol. mabye
            // we could process here too??
            let mut mappings: Vec<KeyMapping> = Vec::new();
            if *layer > 0 {
                // specific layer
                let _ = keyboard.send(&messages::Messages::read_config(
                    device_info.num_keys,
                    device_info.num_encoders,
                    *layer,
                ));
                // read keys for specified layer
                info!("reading keys for layer {}", layer);
                let data = messages::Messages::read_config(
                    device_info.num_keys,
                    device_info.num_encoders,
                    *layer,
                );
                let _ = keyboard.send(&data);

                // read all messages from device
                loop {
                    let bytes_read = keyboard.recieve(&mut buf)?;
                    if bytes_read == 0 {
                        break;
                    }
                    debug!("bytes read: {bytes_read}");
                    debug!("data: {:02x?}", buf);
                    mappings.push(decoder::Decoder::get_key_mapping(&buf)?);
                }
            } else {
                // read keys for all layers
                for i in 1..=consts::NUM_LAYERS {
                    let _ = keyboard.send(&messages::Messages::read_config(
                        device_info.num_keys,
                        device_info.num_encoders,
                        i,
                    ));
                    info!("reading keys for layer {i}");
                    let data = messages::Messages::read_config(
                        device_info.num_keys,
                        device_info.num_encoders,
                        i,
                    );
                    let _ = keyboard.send(&data);

                    // read all messages from device
                    loop {
                        let bytes_read = keyboard.recieve(&mut buf)?;
                        if bytes_read == 0 {
                            break;
                        }
                        debug!("bytes read: {bytes_read}");
                        debug!("data: {:02x?}", buf);
                        mappings.push(decoder::Decoder::get_key_mapping(&buf)?);
                    }
                }
            }

            // process responses from device
            let rows_cols = guestimate_rows_cols(device_info.num_keys)?;
            let mut mp = Macropad::new(rows_cols.0, rows_cols.1, device_info.num_encoders);
            let mut knob_idx = 0;
            let mut knob_type = 0;
            let mut last_layer = 0;
            for km in mappings {
                debug!("{:?}", km);
                if km.layer != last_layer {
                    last_layer = km.layer;
                    knob_idx = 0;
                    knob_type = 0;
                }

                if km.key_number <= mp.device.rows * mp.device.cols {
                    // button mappings
                    let row_col = get_position(&mp, km.key_number)?;
                    debug!(
                        "   key: {} at row: {} col: {}",
                        km.key_number, row_col.0, row_col.1
                    );
                    mp.layers[(km.layer - 1) as usize].buttons[row_col.0][row_col.1] =
                        km.keys.join("-");
                } else {
                    // knobs
                    debug!("knob idx: {} knob type: {}", knob_idx, knob_type);
                    match knob_type {
                        0 => {
                            mp.layers[(km.layer - 1) as usize].knobs[knob_idx].ccw =
                                km.keys.join("-");
                            knob_type += 1;
                        }
                        1 => {
                            mp.layers[(km.layer - 1) as usize].knobs[knob_idx].press =
                                km.keys.join("-");
                            knob_type += 1;
                        }
                        2 => {
                            mp.layers[(km.layer - 1) as usize].knobs[knob_idx].cw =
                                km.keys.join("-");
                            knob_type = 0;
                            knob_idx += 1;
                        }
                        _ => {
                            panic!("should not get here!")
                        }
                    }
                }
            }
            Mapping::print(mp);
        }
    }

    Ok(())
}

pub fn get_position(mp: &Macropad, key_num: u8) -> Result<(usize, usize)> {
    let cols = mp.device.cols;
    let mut col;
    let mut row;

    if key_num % cols == 0 {
        row = key_num / cols;
        if row > 0 {
            row -= 1;
        }
    } else {
        row = key_num / cols;
    }
    if key_num > cols {
        col = key_num % cols;
        if col == 0 {
            col = cols;
        }
        col -= 1;
    } else {
        col = key_num - 1;
    }
    Ok((row.into(), col.into()))
}

pub fn guestimate_rows_cols(num_keys: u8) -> Result<(u8, u8)> {
    match num_keys {
        6 => Ok((2, 3)),
        9 => Ok((3, 3)),
        12 => Ok((3, 4)),
        _ => Err(anyhow!("unable to guess rows/cols for {num_keys}")),
    }
}

pub fn find_interface_and_endpoint(
    device: &Device<Context>,
    interface_num: Option<u8>,
    endpoint_addr_out: u8,
    endpoint_addr_in: u8,
) -> Result<(u8, u8, u8)> {
    let conf_desc = device
        .config_descriptor(0)
        .context("get config #0 descriptor")?;

    // Get the numbers of interfaces to explore
    let interface_nums = match interface_num {
        Some(iface_num) => vec![iface_num],
        None => conf_desc.interfaces().map(|iface| iface.number()).collect(),
    };

    for iface_num in interface_nums {
        debug!("Probing interface {iface_num}");

        // Look for an interface with the given number
        let intf = conf_desc
            .interfaces()
            .find(|iface| iface_num == iface.number())
            .ok_or_else(|| {
                anyhow!(
                    "interface #{} not found, interface numbers:\n{:#?}",
                    iface_num,
                    conf_desc.interfaces().map(|i| i.number()).format(", ")
                )
            })?;

        // Check that it's a HID device
        let intf_desc = intf.descriptors().exactly_one().map_err(|_| {
            anyhow!(
                "only one interface descriptor is expected, got:\n{:#?}",
                intf.descriptors().format("\n")
            )
        })?;
        ensure!(
            intf_desc.class_code() == 0x03
                && intf_desc.sub_class_code() == 0x00
                && intf_desc.protocol_code() == 0x00,
            "unexpected interface parameters: {:#?}",
            intf_desc
        );

        let descriptors = intf_desc.endpoint_descriptors();
        // per usb spec, the max value for a usb endpoint is 7 bits (or 127)
        // so set the values to be invalid by default
        let mut out_if = 0xFF;
        let mut in_if = 0xFF;
        for endpoint in descriptors {
            debug!("==> {:?} direction: {:?}", endpoint, endpoint.direction());
            if endpoint.transfer_type() == TransferType::Interrupt
                && endpoint.direction() == Direction::Out
                && endpoint.address() == endpoint_addr_out
            {
                out_if = endpoint.address();
            }
            if endpoint.transfer_type() == TransferType::Interrupt
                && endpoint.direction() == Direction::In
                && endpoint.address() == endpoint_addr_in
            {
                in_if = endpoint.address();
            }
            if out_if < 0xFF && in_if < 0xFF {
                debug!("Found endpoint {endpoint:?}");
                return Ok((iface_num, out_if, in_if));
            }
        }
    }

    Err(anyhow!("No valid interface/endpoint combination found!"))
}

fn open_keyboard(options: &Options) -> Result<Box<dyn Keyboard>> {
    // Find USB device based on the product id
    let (device, desc, id_product) = find_device(
        options.devel_options.vendor_id,
        options.devel_options.product_id,
    )
    .context("find USB device")?;

    ensure!(
        desc.num_configurations() == 1,
        "only one device configuration is expected"
    );

    // Find correct endpoint
    let (intf_num, endpt_addr_out, endpt_addr_in) = find_interface_and_endpoint(
        &device,
        options.devel_options.interface_number,
        options.devel_options.out_endpoint_address,
        options.devel_options.in_endpoint_address,
    )?;

    // Open device.
    let mut handle = device.open().context("open USB device")?;
    let _ = handle.set_auto_detach_kernel_driver(true);
    handle
        .claim_interface(intf_num)
        .context("claim interface")?;

    match id_product {
        0x8840 | 0x8842 => k884x::Keyboard884x::new(handle, endpt_addr_out, endpt_addr_in)
            .map(|v| Box::new(v) as Box<dyn Keyboard>),
        0x8880 => k8880::Keyboard8880::new(handle, endpt_addr_out, endpt_addr_in)
            .map(|v| Box::new(v) as Box<dyn Keyboard>),
        _ => unreachable!("This shouldn't happen!"),
    }
}

pub fn find_device(vid: u16, pid: u16) -> Result<(Device<Context>, DeviceDescriptor, u16)> {
    let options = vec![
        #[cfg(windows)]
        rusb::UsbOption::use_usbdk(),
    ];
    let usb_context = rusb::Context::with_options(&options)?;

    let mut found = vec![];
    for device in usb_context.devices().context("get USB device list")?.iter() {
        let desc = device.device_descriptor().context("get USB device info")?;
        debug!(
            "Bus {:03} Device {:03} ID {:04x}:{:04x}",
            device.bus_number(),
            device.address(),
            desc.vendor_id(),
            desc.product_id()
        );
        let product_id = desc.product_id();

        if desc.vendor_id() == vid && PRODUCT_IDS.contains(&pid) {
            found.push((device, desc, product_id));
        }
    }

    match found.len() {
        0 => Err(anyhow!(
            "macropad device not found. Use --vendor-id and --product-id to override defaults"
        )),
        1 => Ok(found.pop().unwrap()),
        _ => {
            let mut addresses = vec![];
            for (device, _desc, _product_id) in found {
                let address = (device.bus_number(), device.address());
                addresses.push(address);
            }

            Err(anyhow!(
                indoc! {"
                Several compatible devices are found.
                Unfortunately, this model of keyboard doesn't have serial number.
                So specify USB address using --address option.
                
                Addresses:
                {}
            "},
                addresses
                    .iter()
                    .map(|(bus, addr)| format!("{bus}:{addr}"))
                    .join("\n")
            ))
        }
    }
}
