#!/bin/bash
# A simple script to help run the kernel in QEMU with UEFI

KERNEL_BINARY=$1
OVMF_PATH="/usr/share/ovmf/OVMF.fd" # Default path on many Linux distros

if [ ! -f "$OVMF_PATH" ]; then
    # Try other common paths
    if [ -f "/usr/share/edk2/x64/OVMF.4m.fd" ]; then
        OVMF_PATH="/usr/share/edk2/x64/OVMF.4m.fd"
    elif [ -f "/usr/share/OVMF/OVMF_CODE.fd" ]; then
        OVMF_PATH="/usr/share/OVMF/OVMF_CODE.fd"
    elif [ -f "/usr/share/ovmf/x64/OVMF_CODE.fd" ]; then
        OVMF_PATH="/usr/share/ovmf/x64/OVMF_CODE.fd"
    elif [ -f "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd" ]; then
        OVMF_PATH="/usr/share/edk2-ovmf/x64/OVMF_CODE.fd"
    elif [ -f "/usr/share/edk2/x64/OVMF_CODE.fd" ]; then
        OVMF_PATH="/usr/share/edk2/x64/OVMF_CODE.fd"
    else
        echo "Error: OVMF.fd not found."
        echo "Please install OVMF. On Arch: sudo pacman -S edk2-ovmf"
        echo "On Ubuntu: sudo apt install ovmf"
        exit 1
    fi
fi

# In UEFI, the binary should ideally be an EFI application.
# For now, we are building a raw ELF. bootloader_api v0.11 kernels are usually combined 
# with the bootloader crate in a separate builder.

echo "Kernel Binary: $KERNEL_BINARY"
echo "Note: Direct booting of ELF as a raw drive might not work without a UEFI-compatible GPT image."
echo "Running in QEMU with OVMF..."

# Attempt to run with QEMU. Note that for true UEFI testing, 
# a proper disk image (GPT + FAT32) is recommended.
qemu-system-x86_64 \
    -bios $OVMF_PATH \
    -drive format=raw,file=$KERNEL_BINARY \
    -serial stdio \
    -display gtk
