use crate::consts;
use anyhow::Result;
use log::debug;
use rusb::{Context, DeviceHandle, Error::Timeout};

pub struct Reader {}

impl Reader {
    pub fn read_device_msg(
        endpoint_in_addr: u8,
        handle: &DeviceHandle<Context>,
        buf: &mut Vec<u8>,
    ) -> Result<usize> {
        let read = handle.read_interrupt(endpoint_in_addr, buf, consts::TIMEOUT);

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
