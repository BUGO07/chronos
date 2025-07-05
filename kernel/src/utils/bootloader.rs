/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

pub use limine;

use limine::{
    file::File,
    framebuffer::Framebuffer,
    memory_map::Entry,
    request::{
        BootloaderInfoRequest, DeviceTreeBlobRequest, ExecutableAddressRequest,
        ExecutableFileRequest, FramebufferRequest, HhdmRequest, MemoryMapRequest, MpRequest,
        RequestsEndMarker, RequestsStartMarker, RsdpRequest,
    },
    response::{BootloaderInfoResponse, ExecutableAddressResponse, MpResponse},
};

#[used]
#[unsafe(link_section = ".requests_start_marker")]
pub static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
pub static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static EXECUTABLE_FILE_REQUEST: ExecutableFileRequest = ExecutableFileRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static MP_REQUEST: MpRequest = MpRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static BOOTLOADER_INFO_REQUEST: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static DEVICE_TREE_REQUEST: DeviceTreeBlobRequest = DeviceTreeBlobRequest::new();

pub fn get_framebuffers() -> impl Iterator<Item = Framebuffer<'static>> {
    FRAMEBUFFER_REQUEST
        .get_response()
        .into_iter()
        .flat_map(|x| x.framebuffers())
}

pub fn get_memory_map() -> &'static [&'static Entry] {
    MEMORY_MAP_REQUEST.get_response().unwrap().entries()
}

pub fn get_hhdm_offset() -> u64 {
    HHDM_REQUEST.get_response().unwrap().offset()
}

pub fn get_executable_address() -> &'static ExecutableAddressResponse {
    EXECUTABLE_ADDRESS_REQUEST.get_response().unwrap()
}

pub fn get_executable_file() -> &'static File {
    EXECUTABLE_FILE_REQUEST.get_response().unwrap().file()
}

pub fn get_mp_response() -> &'static MpResponse {
    MP_REQUEST.get_response().unwrap()
}

pub fn get_rsdp_address() -> usize {
    let rsdp = RSDP_REQUEST.get_response().unwrap().address();
    let hddm = get_hhdm_offset() as usize;
    if rsdp < hddm { rsdp } else { rsdp - hddm }
}

pub fn get_bootloader_info() -> &'static BootloaderInfoResponse {
    BOOTLOADER_INFO_REQUEST.get_response().unwrap()
}

pub fn get_device_tree() -> Option<*const ()> {
    DEVICE_TREE_REQUEST.get_response().map(|x| x.dtb_ptr())
}
