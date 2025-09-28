mod config;
mod consts;
mod decoder;
mod keyboard;
mod mapping;
mod options;
mod parse;

use crate::consts::PRODUCT_IDS;
use crate::decoder::Decoder;
use crate::keyboard::{
    k884x, k8890, Keyboard, MediaCode, Modifier, MouseAction, MouseButton, WellKnownCode,
};
use crate::mapping::Macropad;
use crate::options::Options;
use crate::options::{Command, LedCommand};

use anyhow::{anyhow, ensure, Result};
use indoc::indoc;
use itertools::Itertools;
use keyboard::LedColor;
use log::debug;
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
    debug!("options: {:?}", options.devel_options);

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

        Command::Validate {
            config_file,
            product_id,
            device_connected,
        } => {
            if *device_connected {
                debug!("validating with connected device");
                if let Ok(device) = find_device(consts::VENDOR_ID, None) {
                    // read the config for buttons/knobs and validate against file
                    if device.2 != 0x8890 {
                        // 0x8890 does not support reading configuration
                        let mut keyboard = open_keyboard(&options).context("opening keyboard")?;
                        let mut buf = vec![0; consts::READ_BUF_SIZE.into()];

                        // get the type of device
                        keyboard.send(&keyboard.device_type())?;
                        let bytes_read = keyboard.recieve(&mut buf)?;
                        if bytes_read == 0 {
                            return Err(anyhow!(
                                "Unable to read from device to validate mappings. Please use -p option instead to specify your device."
                            ));
                        }
                        let device_info = Decoder::get_device_info(&buf);
                        debug!(
                            "keys: {} encoders: {}",
                            device_info.num_keys, device_info.num_encoders
                        );

                        let macropad = Mapping::read(config_file);
                        if device_info.num_keys != macropad.device.rows * macropad.device.cols {
                            return Err(anyhow!(
                                "Number of keys specified in config does not match device"
                            ));
                        }
                        if device_info.num_encoders != macropad.device.knobs {
                            return Err(anyhow!(
                                "Number of knobs specified in config does not match device"
                            ));
                        }
                    }
                    Mapping::validate(config_file, Some(device.2))
                        .context("validating configuration file with connected device")?;
                    println!("config is valid 👌")
                } else {
                    return Err(anyhow!(
                        "Unable to find connected device with vendor id: 0x{:02x}",
                        consts::VENDOR_ID
                    ));
                }
            } else if let Some(pid) = product_id {
                debug!("validating with supplied product id 0x{pid:02x}");
                Mapping::validate(config_file, Some(*pid))
                    .context("validating configuration file against specified product id")?;
                println!("config is valid 👌")
            } else {
                // load and validate mapping
                println!("validating general ron formatting - unable to do more granular checking; use -p option to check against device");
                Mapping::validate(config_file, None)
                    .context("generic validation of configuration file")?;
                println!("config is valid 👌")
            }
        }

        Command::Program { config_file } => {
            let config = Mapping::read(config_file);
            let mut keyboard = open_keyboard(&options).context("opening keyboard")?;
            keyboard.program(&config).context("programming macropad")?;
            println!("successfully programmed device");
        }

        Command::Led(LedCommand {
            index,
            layer,
            led_color,
        }) => {
            let mut keyboard = open_keyboard(&options).context("opening keyboard")?;

            // color is not supported on 0x8890 so don't require one to be passed
            let color = if led_color.is_some() {
                led_color.unwrap()
            } else {
                LedColor::Red
            };
            keyboard
                .set_led(*index, *layer, color)
                .context("programming LED on macropad")?;
        }

        Command::Read { layer } => {
            debug!("dev options: {:?}", options.devel_options);
            let mut keyboard = open_keyboard(&options).context("opening keyboard")?;
            let macropad_config = keyboard
                .read_macropad_config(layer)
                .context("reading macropad configuration")?;
            Mapping::print(macropad_config);
        }
    }

    Ok(())
}

pub fn find_interface_and_endpoint(
    device: &Device<Context>,
    interface_num: Option<u8>,
    endpoint_addr_out: Option<u8>,
    endpoint_addr_in: Option<u8>,
) -> Result<(u8, u8, u8)> {
    debug!("out: {endpoint_addr_out:?} in: {endpoint_addr_in:?}");
    let conf_desc = device
        .config_descriptor(0)
        .context("get config #0 descriptor")?;

    // Get the numbers of interfaces to explore
    let interface_nums = match interface_num {
        Some(iface_num) => vec![iface_num],
        None => conf_desc.interfaces().map(|iface| iface.number()).collect(),
    };

    // per usb spec, the max value for a usb endpoint is 7 bits (or 127)
    // so set the values to be invalid by default
    let mut out_if = 0xFF;
    let mut in_if = 0xFF;
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

        let descriptors = intf_desc.endpoint_descriptors();
        for endpoint in descriptors {
            // check packet size
            if endpoint.max_packet_size() != (consts::PACKET_SIZE - 1).try_into()? {
                continue;
            }

            debug!("==> {:?} direction: {:?}", endpoint, endpoint.direction());
            if endpoint.transfer_type() == TransferType::Interrupt
                && endpoint.direction() == Direction::Out
            {
                if let Some(ea) = endpoint_addr_out {
                    if endpoint.address() == ea {
                        debug!("Found OUT endpoint {endpoint:?}");
                        out_if = endpoint.address();
                    }
                } else {
                    debug!("Found OUT endpoint {endpoint:?}");
                    out_if = endpoint.address();
                }
            }
            if endpoint.transfer_type() == TransferType::Interrupt
                && endpoint.direction() == Direction::In
            {
                if let Some(ea) = endpoint_addr_in {
                    if endpoint.address() == ea {
                        debug!("Found IN endpoint {endpoint:?}");
                        in_if = endpoint.address();
                    }
                } else {
                    debug!("Found IN endpoint {endpoint:?}");
                    in_if = endpoint.address();
                }
            }
        }
        debug!("ep OUT addr: 0x{out_if:02x} ep IN addr: 0x{in_if:02x}");
        if out_if < 0xFF && in_if < 0xFF {
            return Ok((iface_num, out_if, in_if));
        } else if out_if < 0xFF {
            return Ok((iface_num, out_if, 0xFF));
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
    let handle = device.open().context("open USB device")?;
    let _ = handle.set_auto_detach_kernel_driver(true);
    handle
        .claim_interface(intf_num)
        .context("claim interface")?;

    match id_product {
        0x8840 | 0x8842 => {
            k884x::Keyboard884x::new(Some(handle), endpt_addr_out, endpt_addr_in, id_product)
                .map(|v| Box::new(v) as Box<dyn Keyboard>)
        }
        0x8890 => k8890::Keyboard8890::new(Some(handle), endpt_addr_out)
            .map(|v| Box::new(v) as Box<dyn Keyboard>),
        _ => unreachable!("This shouldn't happen!"),
    }
}

pub fn find_device(vid: u16, pid: Option<u16>) -> Result<(Device<Context>, DeviceDescriptor, u16)> {
    debug!("vid: 0x{vid:02x}");
    if let Some(prod_id) = pid {
        debug!("pid: 0x{prod_id:02x}");
    } else {
        debug!("pid: None");
    }
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

        if desc.vendor_id() == vid {
            if let Some(prod_id) = pid {
                if PRODUCT_IDS.contains(&prod_id) {
                    found.push((device, desc, product_id));
                }
            } else {
                found.push((device, desc, product_id));
            }
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
