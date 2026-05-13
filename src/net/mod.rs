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

            // RTL8139は32ビットDMAのみ対応しているため、4GB以下のメモリが必要
            let rx_buf_phys_64 = rx_frames[0].start_address().as_u64();
            if rx_buf_phys_64 > 0xFFFF_FFFF {
                panic!("RTL8139 RX buffer allocated above 4GB: 0x{:x}", rx_buf_phys_64);
            }
            let rx_buf_phys = rx_buf_phys_64 as u32;
            let rx_buf_virt = (phys_mem_offset + rx_buf_phys_64).as_mut_ptr();

            // TXバッファの確保 (1フレーム = 4KB, 4つのTX記述子に十分)
            let tx_frame = frame_allocator.allocate_frame().expect("No frame for TX buffers");
            let tx_buf_phys_64 = tx_frame.start_address().as_u64();
            if tx_buf_phys_64 > 0xFFFF_FFFF {
                panic!("RTL8139 TX buffer allocated above 4GB: 0x{:x}", tx_buf_phys_64);
            }
            let tx_buf_phys = tx_buf_phys_64 as u32;
            let tx_buf_virt = (phys_mem_offset + tx_buf_phys_64).as_mut_ptr();
            
            let rtl = rtl8139::Rtl8139::new(&dev, rx_buf_phys, rx_buf_virt, tx_buf_phys, tx_buf_virt);
            *RTL8139_DEVICE.lock() = Some(rtl);
            return;
        }
    }
}
