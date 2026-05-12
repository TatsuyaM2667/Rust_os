pub mod rtl8139;
pub mod wifi;

use alloc::vec::Vec;
use crate::pci;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::{VirtAddr, structures::paging::{Mapper, FrameAllocator, Size4KiB}};

lazy_static! {
    pub static ref RTL8139_DEVICE: Mutex<Option<rtl8139::Rtl8139>> = Mutex::new(None);
}

pub fn init(
    _mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    phys_mem_offset: VirtAddr,
) {
    let devices = pci::scan_bus();
    for dev in devices {
        if dev.vendor_id == 0x10ec && dev.device_id == 0x8139 {
            println!("Found RTL8139 at bus {}, device {}", dev.bus, dev.device);
            
            // RXバッファの確保 (32KB)
            let num_rx_frames = (rtl8139::RX_BUF_SIZE + 4095) / 4096;
            let mut rx_frames = Vec::new();
            for _ in 0..num_rx_frames {
                rx_frames.push(frame_allocator.allocate_frame().expect("No frames for RX buffer"));
            }

            // 本来は連続した領域が必要。BootInfoアロケータは通常連続して確保する。
            let rx_buf_phys = rx_frames[0].start_address().as_u64() as u32;
            let rx_buf_virt = (phys_mem_offset + rx_buf_phys as u64).as_mut_ptr();

            // TXバッファの確保 (1フレーム = 4KB, 4つのTX記述子に十分)
            let tx_frame = frame_allocator.allocate_frame().expect("No frame for TX buffers");
            let tx_buf_phys = tx_frame.start_address().as_u64() as u32;
            let tx_buf_virt = (phys_mem_offset + tx_buf_phys as u64).as_mut_ptr();
            
            let rtl = rtl8139::Rtl8139::new(&dev, rx_buf_phys, rx_buf_virt, tx_buf_phys, tx_buf_virt);
            *RTL8139_DEVICE.lock() = Some(rtl);
            return;
        }
    }
}
