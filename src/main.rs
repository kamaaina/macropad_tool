mod config;
mod consts;
mod decoder;
mod keyboard;
mod messages;
mod options;
mod parse;

use crate::config::Config;
use crate::consts::PRODUCT_IDS;
use crate::decoder::KeyMapping;
use crate::keyboard::{
    k884x, k8880, Keyboard, KnobAction, MediaCode, Modifier, MouseAction, MouseButton,
    WellKnownCode,
};
use crate::messages::Messages;
use crate::options::{Command, LedCommand};
use crate::{keyboard::Key, options::Options};

use anyhow::{anyhow, ensure, Result};
use indoc::indoc;
use itertools::Itertools;
use log::{debug, info};
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

        Command::Validate => {
            // Load and validate mapping.
            let config: Config =
                serde_yaml::from_reader(std::io::stdin().lock()).context("load mapping config")?;
            let _ = config.render().context("render mappings config")?;
            println!("config is valid 👌")
        }

        Command::Program => {
            // Load and validate mapping.
            let config: Config =
                serde_yaml::from_reader(std::io::stdin().lock()).context("load mapping config")?;
            let layers = config.render().context("render mapping config")?;

            let mut keyboard = open_keyboard(&options)?;

            // Apply keyboard mapping.
            for (layer_idx, layer) in layers.iter().enumerate() {
                for (button_idx, macro_) in layer.buttons.iter().enumerate() {
                    if let Some(macro_) = macro_ {
                        keyboard
                            .bind_key(layer_idx as u8, Key::Button(button_idx as u8), macro_)
                            .context("bind key")?;
                    }
                }

                for (knob_idx, knob) in layer.knobs.iter().enumerate() {
                    if let Some(macro_) = &knob.ccw {
                        keyboard.bind_key(
                            layer_idx as u8,
                            Key::Knob(knob_idx as u8, KnobAction::RotateCCW),
                            macro_,
                        )?;
                    }
                    if let Some(macro_) = &knob.press {
                        keyboard.bind_key(
                            layer_idx as u8,
                            Key::Knob(knob_idx as u8, KnobAction::Press),
                            macro_,
                        )?;
                    }
                    if let Some(macro_) = &knob.cw {
                        keyboard.bind_key(
                            layer_idx as u8,
                            Key::Knob(knob_idx as u8, KnobAction::RotateCW),
                            macro_,
                        )?;
                    }
                }
            }
            let _ = keyboard.send(&Messages::end_program());

            println!("デバイスのプログラミングが完了しました");
        }

        Command::Led(LedCommand { index }) => {
            let mut keyboard = open_keyboard(&options)?;
            keyboard.set_led(*index)?;
        }

        Command::Read { layer, mapping } => {
            println!("dev options: {:?}", options.devel_options);
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
            for km in mappings {
                println!("{:?}", km);
            }

            if mapping.is_empty() {
                // FIXME: write configuration to stdout
                info!("write configuration to stdout");
            } else {
                // FIXME: write to file in yaml format
                info!("write configuration to file: {mapping}");
            }
        }
    }

    Ok(())
}

/*
fn find_and_init(options: &Options) -> Result<DeviceHandle<Context>> {
    // find USB device based on the product id
    let (device, _desc, _id_product) = find_device(
        options.devel_options.vendor_id,
        options
            .devel_options
            .product_id
            .expect("expected product id"),
    )
    .context("find USB device")?;

    // find correct endpoints; we need both IN and OUT
    let (intf_num, endpt_addr_out, endpt_addr_in) = find_interface_and_endpoint(
        &device,
        None,
        options.devel_options.out_endpoint_address,
        options.devel_options.in_endpoint_address,
    )?;
    info!(
        "found interface number: {intf_num}, OUT endpoint: 0x{:02x}, and IN endpoint: 0x{:02x}",
        endpt_addr_out, endpt_addr_in
    );

    // open device
    let mut handle = device.open().context("open USB device")?;
    let _ = handle.set_auto_detach_kernel_driver(true);
    handle
        .claim_interface(intf_num)
        .context("claim interface")?;

    let mut buf = vec![0; consts::READ_BUF_SIZE.into()];

    // probe device to see what type of macropad we have
    let mp_type = messages::Messages::device_type();
    let _ = handle.write_interrupt(endpt_addr_out, &mp_type, consts::TIMEOUT);
    let _ = reader::Reader::read_device_msg(
        options.devel_options.in_endpoint_address,
        &handle,
        &mut buf,
    );

    Ok(handle)
}
*/

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
        options
            .devel_options
            .product_id
            .expect("expected product id"),
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

        /*
        // FIXME: add support for other product id's
        if desc.vendor_id() == vid && desc.product_id() == pid {
            found.push((device, desc, product_id));
        }
        */
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
