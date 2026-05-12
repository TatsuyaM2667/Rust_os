use x86_64::instructions::port::Port;
use crate::pci::PciDevice;
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;

const REG_MAC: u16 = 0x00;
const REG_TSD0: u16 = 0x10;
const REG_TSAD0: u16 = 0x20;
const REG_RBSTART: u16 = 0x30;
const REG_CR: u16 = 0x37;
const REG_CAPR: u16 = 0x38;
const REG_IMR: u16 = 0x3C;
const REG_ISR: u16 = 0x3E;
const REG_RCR: u16 = 0x44;
const REG_CONFIG1: u16 = 0x52;

const CR_RESET: u8 = 0x10;
const CR_RE: u8 = 0x08;
const CR_TE: u8 = 0x04;
const CR_BUFE: u8 = 0x01;

const RCR_AAP: u32 = 0x01;
const RCR_APM: u32 = 0x02;
const RCR_AM: u32 = 0x04;
const RCR_AB: u32 = 0x08;
const RCR_WRAP: u32 = 0x80;

pub const RX_BUF_SIZE: usize = 8192 + 16 + 1500;

pub struct Rtl8139 {
    io_base: u16,
    mac_addr: [u8; 6],
    rx_buf_phys: u32,
    rx_buf_virt: *mut u8,
    rx_offset: usize,
    tx_buf_phys: u32,
    tx_buf_virt: *mut u8,
    tx_index: usize,
}

unsafe impl Send for Rtl8139 {}
unsafe impl Sync for Rtl8139 {}

impl Rtl8139 {
    pub fn new(device: &PciDevice, rx_buf_phys: u32, rx_buf_virt: *mut u8, tx_buf_phys: u32, tx_buf_virt: *mut u8) -> Self {
        let io_base = (device.base_addr_0 & 0xFFFC) as u16;
        
        let mut rtl = Rtl8139 {
            io_base,
            mac_addr: [0; 6],
            rx_buf_phys,
            rx_buf_virt,
            rx_offset: 0,
            tx_buf_phys,
            tx_buf_virt,
            tx_index: 0,
        };

        rtl.init();
        rtl
    }

    pub fn mac_address(&self) -> [u8; 6] {
        self.mac_addr
    }

    fn init(&mut self) {
        unsafe {
            Port::<u8>::new(self.io_base + REG_CONFIG1).write(0x00);
            
            let mut cr_port = Port::<u8>::new(self.io_base + REG_CR);
            cr_port.write(CR_RESET);
            while (cr_port.read() & CR_RESET) != 0 {}

            Port::<u32>::new(self.io_base + REG_RBSTART).write(self.rx_buf_phys);
            Port::<u16>::new(self.io_base + REG_IMR).write(0x0005);
            Port::<u32>::new(self.io_base + REG_RCR).write(RCR_AAP | RCR_APM | RCR_AM | RCR_AB | RCR_WRAP);
            Port::<u8>::new(self.io_base + REG_CR).write(CR_RE | CR_TE);

            for i in 0..6 {
                self.mac_addr[i] = Port::<u8>::new(self.io_base + REG_MAC + i as u16).read();
            }
        }
    }

    pub fn handle_interrupt(&mut self) {
        unsafe {
            let mut isr_port = Port::<u16>::new(self.io_base + REG_ISR);
            let status = isr_port.read();
            isr_port.write(status);
        }
    }
}

impl Device for Rtl8139 {
    type RxToken<'a> = Rtl8139RxToken;
    type TxToken<'a> = Rtl8139TxToken;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        unsafe {
            let cr = Port::<u8>::new(self.io_base + REG_CR).read();
            if (cr & CR_BUFE) != 0 {
                return None;
            }
        }

        Some((
            Rtl8139RxToken {
                io_base: self.io_base,
                rx_buf_virt: self.rx_buf_virt,
                rx_offset_ptr: &mut self.rx_offset as *mut usize,
            },
            Rtl8139TxToken {
                io_base: self.io_base,
                tx_buf_phys: self.tx_buf_phys,
                tx_buf_virt: self.tx_buf_virt,
                tx_index_ptr: &mut self.tx_index as *mut usize,
            }
        ))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(Rtl8139TxToken {
            io_base: self.io_base,
            tx_buf_phys: self.tx_buf_phys,
            tx_buf_virt: self.tx_buf_virt,
            tx_index_ptr: &mut self.tx_index as *mut usize,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub struct Rtl8139RxToken {
    io_base: u16,
    rx_buf_virt: *mut u8,
    rx_offset_ptr: *mut usize,
}

impl RxToken for Rtl8139RxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        unsafe {
            let offset = *self.rx_offset_ptr;
            let rx_ptr = self.rx_buf_virt.add(offset);
            let header = *(rx_ptr as *const u32);
            let length = (header >> 16) as usize;
            
            let data = core::slice::from_raw_parts_mut(rx_ptr.add(4), length - 4);
            let result = f(data);

            *self.rx_offset_ptr = (offset + length + 4 + 3) & !3;
            Port::<u16>::new(self.io_base + REG_CAPR).write((*self.rx_offset_ptr as u16).wrapping_sub(16));

            result
        }
    }
}

pub struct Rtl8139TxToken {
    io_base: u16,
    tx_buf_phys: u32,
    tx_buf_virt: *mut u8,
    tx_index_ptr: *mut usize,
}

impl TxToken for Rtl8139TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        unsafe {
            let tx_idx = *self.tx_index_ptr;
            let tx_ptr = self.tx_buf_virt.add(tx_idx * 2048);
            let buf = core::slice::from_raw_parts_mut(tx_ptr, 2048);
            
            let result = f(&mut buf[..len]);

            let tx_phys = self.tx_buf_phys + (tx_idx * 2048) as u32;
            Port::<u32>::new(self.io_base + REG_TSAD0 + (tx_idx * 4) as u16).write(tx_phys);
            Port::<u32>::new(self.io_base + REG_TSD0 + (tx_idx * 4) as u16).write(len as u32 & 0x1FFF);

            *self.tx_index_ptr = (tx_idx + 1) % 4;
            result
        }
    }
}
