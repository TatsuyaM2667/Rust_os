#!/bin/bash
# A simple script to help run the kernel in QEMU with UEFI
set -e

# 絶対パスに変換
KERNEL_BINARY=$(realpath "$1")
UEFI_IMAGE="${KERNEL_BINARY}_uefi.img"
OVMF_PATH="/usr/share/ovmf/OVMF.fd"

# OVMFのパス検出
for path in "/usr/share/ovmf/OVMF.fd" "/usr/share/edk2/x64/OVMF.4m.fd" "/usr/share/OVMF/OVMF_CODE.fd" "/usr/share/ovmf/x64/OVMF_CODE.fd" "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd" "/usr/share/edk2/x64/OVMF_CODE.fd"; do
    if [ -f "$path" ]; then
        OVMF_PATH="$path"
        break
    fi
done

# イメージファイルを一旦削除
rm -f "$UEFI_IMAGE"

# ビルダーを実行してイメージを作成
echo "Converting ELF to UEFI boot image..."

if [ -d ".cargo" ]; then
    mv .cargo .cargo_temp
    trap 'mv .cargo_temp .cargo' EXIT
fi

# builderディレクトリで実行（絶対パスを渡す）
(cd builder && cargo run -- "$KERNEL_BINARY")

if [ -d ".cargo_temp" ]; then
    mv .cargo_temp .cargo
    trap - EXIT
fi

if [ ! -f "$UEFI_IMAGE" ]; then
    echo "Error: Failed to create UEFI image at $UEFI_IMAGE"
    exit 1
fi

echo "Running in QEMU..."
qemu-system-x86_64 \
    -bios "$OVMF_PATH" \
    -drive format=raw,file="$UEFI_IMAGE" \
    -serial stdio \
    -display gtk
