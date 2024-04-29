mod config;
mod consts;
mod decoder;
mod keyboard;
mod mapping;
mod options;
mod parse;

use crate::consts::PRODUCT_IDS;
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
            let mut keyboard = open_keyboard(&options).context("opening keyboard")?;
            keyboard.program(&config).context("programming macropad")?;
            println!("デバイスのプログラミングが完了しました");
        }

        Command::Led(LedCommand { index, led_color }) => {
            let mut keyboard = open_keyboard(&options).context("opening keyboard")?;

            // color is not supported on 0x8890 so don't require one to be passed
            let color = if led_color.is_some() {
                led_color.unwrap()
            } else {
                LedColor::Red
            };
            keyboard
                .set_led(*index, color)
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
            } else if out_if < 0xFF {
                debug!("Found OUT endpoint {endpoint:?}");
                return Ok((iface_num, out_if, 0xFF));
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
        0x8840 | 0x8842 => k884x::Keyboard884x::new(Some(handle), endpt_addr_out, endpt_addr_in)
            .map(|v| Box::new(v) as Box<dyn Keyboard>),
        0x8890 => k8890::Keyboard8890::new(Some(handle), endpt_addr_out)
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
