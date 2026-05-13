use bootloader::UefiBoot;
use std::path::Path;

fn main() {
    let kernel_path = Path::new("target/x86_64-unknown-none/debug/rust_os");
    let out_path = Path::new("target/x86_64-unknown-none/debug/rust_os_uefi.img");
    
    if !kernel_path.exists() {
        panic!("Kernel binary not found. Run 'cargo build' first.");
    }

    let mut boot = UefiBoot::new(kernel_path);
    boot.create_disk_image(out_path).expect("Failed to create UEFI disk image");
    
    println!("UEFI disk image created at: {:?}", out_path);
}
