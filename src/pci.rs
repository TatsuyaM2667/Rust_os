use x86_64::instructions::port::Port;

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub base_addr_0: u32,
}

pub fn read_config_u32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (offset as u32 & 0xFC)
        | 0x8000_0000;

    let mut addr_port = Port::new(CONFIG_ADDRESS);
    let mut data_port = Port::new(CONFIG_DATA);

    unsafe {
        addr_port.write(address);
        data_port.read()
    }
}

pub fn scan_bus() -> alloc::vec::Vec<PciDevice> {
    use alloc::vec::Vec;

    let mut devices = Vec::new();

    for bus in 0..255 {
        for device in 0..32 {
            let vendor_id = (read_config_u32(bus, device, 0, 0) & 0xFFFF) as u16;
            if vendor_id == 0xFFFF {
                continue;
            }

            let device_id = (read_config_u32(bus, device, 0, 0) >> 16) as u16;
            let base_addr_0 = read_config_u32(bus, device, 0, 0x10);

            devices.push(PciDevice {
                bus,
                device,
                vendor_id,
                device_id,
                base_addr_0,
            });
        }
    }
    devices
}
