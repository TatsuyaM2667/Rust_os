use std::env;
use std::path::PathBuf;

fn main() {
    let kernel_binary = env::args().nth(1).expect("No kernel binary provided");
    let kernel_path = PathBuf::from(kernel_binary);
    
    // カーネルのバイナリがある場所に .img を作成する
    let uefi_path = PathBuf::from(format!("{}_uefi.img", kernel_path.display()));

    // UEFIイメージの作成
    let bootloader_config = bootloader::UefiBoot::new(&kernel_path);
    bootloader_config.create_disk_image(&uefi_path).unwrap();

    println!("Created UEFI boot image at: {}", uefi_path.display());
}
