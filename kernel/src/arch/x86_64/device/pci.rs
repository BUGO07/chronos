/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use x86_64::instructions::port::Port;

use crate::info;

pub static PCI_DEVICES: OnceCell<Vec<PciDevice>> = OnceCell::uninit();

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
    fn config_address(&self, offset: u8) -> u32 {
        let bus = self.bus as u32;
        let device = self.device as u32;
        let function = self.function as u32;
        let offset = offset as u32 & 0xFC;
        0x8000_0000 | (bus << 16) | (device << 11) | (function << 8) | offset
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
    let address = addr.config_address(offset);
    unsafe {
        let mut port_cf8 = Port::<u32>::new(0xCF8);
        let mut port_cfc = Port::<u32>::new(0xCFC);
        port_cf8.write(address);
        port_cfc.read()
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
    let address = addr.config_address(offset);
    unsafe {
        let mut port_cf8 = Port::<u32>::new(0xCF8);
        let mut port_cfc = Port::<u32>::new(0xCFC);
        port_cf8.write(address);
        port_cfc.write(value);
    }
}

//TODO: make it faster with mmio
pub fn pci_enumerate() {
    let mut devices = Vec::new();
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

                devices.push(PciDevice {
                    address: PciAddress {
                        bus,
                        device,
                        function,
                    },
                    vendor_id,
                    device_id,
                    class_code,
                    subclass,
                    prog_if,
                });

                if function == 0 && ((pci_config_read_u32(pciaddr, 0x0C) >> 16) & 0x80) == 0 {
                    break;
                }
            }
        }
    }
    PCI_DEVICES.init_once(|| devices);
}
