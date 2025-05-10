/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::vec::Vec;

use crate::info;

#[cfg(target_arch = "x86_64")]
use crate::utils::asm::port::{inl, outl};

pub static mut PCI_DEVICES: Vec<PciDevice> = Vec::new();
pub static mut MCFG_ADDRESS: u64 = 0;

#[derive(Debug)]
pub struct PciDevice {
    pub address: PciAddress,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct PciAddress {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciAddress {
    #[cfg(target_arch = "x86_64")]
    fn io_config_address(&self, offset: u8) -> u32 {
        let bus = self.bus as u32;
        let device = self.device as u32;
        let function = self.function as u32;
        let offset = offset as u32 & 0xFC;
        0x8000_0000 | (bus << 16) | (device << 11) | (function << 8) | offset
    }

    fn mmio_config_address(&self, offset: u8) -> *mut u32 {
        let bus = self.bus as u64;
        let device = self.device as u64;
        let function = self.function as u64;
        let offset = (offset as u64) & !0x3;
        unsafe {
            (MCFG_ADDRESS + ((bus << 20) | (device << 15) | (function << 12) | offset)) as *mut u32
        }
    }
}

pub fn pci_config_read_u8(addr: PciAddress, offset: u8) -> u8 {
    let shift = (offset & 3) * 8;
    (pci_config_read_u32(addr, offset) >> shift) as u8
}

pub fn pci_config_read_u16(addr: PciAddress, offset: u8) -> u16 {
    let shift = (offset & 2) * 8;
    (pci_config_read_u32(addr, offset) >> shift) as u16
}

pub fn pci_config_read_u32(addr: PciAddress, offset: u8) -> u32 {
    unsafe {
        if MCFG_ADDRESS != 0 {
            core::ptr::read_volatile(addr.mmio_config_address(offset))
        } else {
            #[cfg(target_arch = "x86_64")]
            {
                let address = addr.io_config_address(offset);
                outl(0xCF8, address);
                inl(0xCFC)
            }

            #[cfg(target_arch = "aarch64")]
            {
                panic!("PCI access via I/O ports is not supported on AArch64");
            }
        }
    }
}

pub fn pci_config_write_u8(addr: PciAddress, offset: u8, value: u8) {
    let orig = pci_config_read_u32(addr, offset);
    let shift = (offset & 3) * 8;
    let mask = !(0xFF << shift);
    let new = (orig & mask) | ((value as u32) << shift);
    pci_config_write_u32(addr, offset, new);
}

pub fn pci_config_write_u16(addr: PciAddress, offset: u8, value: u16) {
    let orig = pci_config_read_u32(addr, offset);
    let shift = (offset & 2) * 8;
    let mask = !(0xFFFF << shift);
    let new = (orig & mask) | ((value as u32) << shift);
    pci_config_write_u32(addr, offset, new);
}

pub fn pci_config_write_u32(addr: PciAddress, offset: u8, value: u32) {
    unsafe {
        if MCFG_ADDRESS != 0 {
            core::ptr::write_volatile(addr.mmio_config_address(offset), value);
        } else {
            #[cfg(target_arch = "x86_64")]
            {
                let address = addr.io_config_address(offset);
                outl(0xCF8, address);
                outl(0xCFC, value);
            }

            #[cfg(target_arch = "aarch64")]
            {
                panic!("PCI access via I/O ports is not supported on AArch64");
            }
        }
    }
}

pub fn pci_enumerate() {
    for bus in 0..=255 {
        for device in 0..32 {
            for function in 0..8 {
                let pciaddr = PciAddress {
                    bus,
                    device,
                    function,
                };
                let vendor_device = pci_config_read_u32(pciaddr, 0x00);
                if vendor_device == 0xFFFF_FFFF {
                    continue;
                }

                let vendor_id = (vendor_device & 0xFFFF) as u16;
                let device_id = ((vendor_device >> 16) & 0xFFFF) as u16;

                let class_info = pci_config_read_u32(pciaddr, 0x08);
                let class_code = ((class_info >> 24) & 0xFF) as u8;
                let subclass = ((class_info >> 16) & 0xFF) as u8;
                let prog_if = ((class_info >> 8) & 0xFF) as u8;

                let device_type = match (class_code, subclass, prog_if) {
                    (0x01, 0x06, 0x01) => "AHCI storage controller",
                    (0x01, 0x08, 0x02) => "NVMe storage device",
                    (0x01, 0x01, _) => "IDE storage controller",
                    (0x02, 0x00, _) => "Ethernet controller",
                    (0x03, 0x00, _) => "VGA-compatible device",
                    (0x03, 0x80, _) => "Other display device",
                    (0x04, 0x03, _) => "Audio device",
                    (0x06, _, _) => "Bridge device",
                    (0x0C, 0x03, _) => "USB Controller",
                    _ => "PCI device",
                };

                info!(
                    "Found {device_type} {vendor_id:04X}:{device_id:04X} [0x{class_code:X}:0x{subclass:X}:0x{prog_if:X}]",
                );

                unsafe {
                    PCI_DEVICES.push(PciDevice {
                        address: pciaddr,
                        vendor_id,
                        device_id,
                        class_code,
                        subclass,
                        prog_if,
                    })
                };

                if function == 0 && ((pci_config_read_u32(pciaddr, 0x0C) >> 16) & 0x80) == 0 {
                    break;
                }
            }
        }
    }
}
