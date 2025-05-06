/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    alloc::Layout,
    ffi::{CStr, c_void},
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use spin::{Mutex, mutex::SpinMutex};
use uacpi_sys::*;

use crate::{
    arch::{
        device::pci::{
            PciAddress, pci_config_read_u8, pci_config_read_u16, pci_config_read_u32,
            pci_config_write_u8, pci_config_write_u16, pci_config_write_u32,
        },
        interrupts::StackFrame,
    },
    debug, error, info,
    memory::vmm::{PAGEMAP, flag, page_size},
    utils::{
        align_down, align_up,
        asm::port::{inb, inl, inw, outb, outl, outw},
        limine::get_rsdp_address,
    },
    warn,
};

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_get_rsdp(out_rsdp_address: *mut uacpi_phys_addr) -> uacpi_status {
    unsafe { *out_rsdp_address = get_rsdp_address() as uacpi_phys_addr };
    UACPI_STATUS_OK
}

static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);
static PCI_HANDLES: Mutex<BTreeMap<usize, PciAddress>> = Mutex::new(BTreeMap::new());

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_pci_device_open(
    address: uacpi_pci_address,
    handle: *mut uacpi_handle,
) -> uacpi_status {
    let pci_addr = PciAddress {
        bus: address.segment as u8,
        device: address.device,
        function: address.function,
    };

    let id = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    PCI_HANDLES.lock().insert(id, pci_addr);
    unsafe { *handle = id as *mut c_void };
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_pci_device_close(handle: uacpi_handle) {
    let id = handle as usize;
    PCI_HANDLES.lock().remove(&id);
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_pci_read8(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: *mut uacpi_u8,
) -> uacpi_status {
    let id = handle as usize;
    if let Some(addr) = PCI_HANDLES.lock().get(&id) {
        unsafe { *value = pci_config_read_u8(*addr, offset as u8) };
        UACPI_STATUS_OK
    } else {
        UACPI_STATUS_INVALID_ARGUMENT
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_pci_read16(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: *mut uacpi_u16,
) -> uacpi_status {
    let id = handle as usize;
    if let Some(addr) = PCI_HANDLES.lock().get(&id) {
        unsafe { *value = pci_config_read_u16(*addr, offset as u8) };
        UACPI_STATUS_OK
    } else {
        UACPI_STATUS_INVALID_ARGUMENT
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_pci_read32(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: *mut uacpi_u32,
) -> uacpi_status {
    let id = handle as usize;
    if let Some(addr) = PCI_HANDLES.lock().get(&id) {
        unsafe { *value = pci_config_read_u32(*addr, offset as u8) };
        UACPI_STATUS_OK
    } else {
        UACPI_STATUS_INVALID_ARGUMENT
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_pci_write8(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u8,
) -> uacpi_status {
    let id = handle as usize;
    if let Some(addr) = PCI_HANDLES.lock().get(&id) {
        pci_config_write_u8(*addr, offset as u8, value);
        UACPI_STATUS_OK
    } else {
        UACPI_STATUS_INVALID_ARGUMENT
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_pci_write16(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u16,
) -> uacpi_status {
    let id = handle as usize;
    if let Some(addr) = PCI_HANDLES.lock().get(&id) {
        pci_config_write_u16(*addr, offset as u8, value);
        UACPI_STATUS_OK
    } else {
        UACPI_STATUS_INVALID_ARGUMENT
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_pci_write32(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u32,
) -> uacpi_status {
    let id = handle as usize;
    if let Some(addr) = PCI_HANDLES.lock().get(&id) {
        pci_config_write_u32(*addr, offset as u8, value);
        UACPI_STATUS_OK
    } else {
        UACPI_STATUS_INVALID_ARGUMENT
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_io_map(
    base: uacpi_io_addr,
    _: uacpi_size,
    handle: uacpi_handle,
) -> uacpi_status {
    unsafe { *(handle as *mut u64) = base }
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_io_unmap(_handle: uacpi_handle) {}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_io_read8(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: *mut uacpi_u8,
) -> uacpi_status {
    unsafe { *value = inb((handle as usize + offset) as u16) };
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_io_read16(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: *mut uacpi_u16,
) -> uacpi_status {
    unsafe { *value = inw((handle as usize + offset) as u16) };
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_io_read32(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: *mut uacpi_u32,
) -> uacpi_status {
    unsafe { *value = inl((handle as usize + offset) as u16) };
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_io_write8(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u8,
) -> uacpi_status {
    outb((handle as usize + offset) as u16, value);
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_io_write16(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u16,
) -> uacpi_status {
    outw((handle as usize + offset) as u16, value);
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_io_write32(
    handle: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u32,
) -> uacpi_status {
    outl((handle as usize + offset) as u16, value);
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_map(addr: uacpi_phys_addr, len: uacpi_size) -> *mut c_void {
    let psize = page_size::SMALL;
    let paddr = align_down(addr, psize);
    let size = align_up((addr - paddr) + len as u64, psize);

    for i in (0..size).step_by(psize as usize) {
        unsafe {
            PAGEMAP.get_mut().unwrap().lock().map(
                paddr + i,
                paddr + i,
                flag::PRESENT | flag::WRITE,
                psize,
            )
        };
    }
    addr as *mut c_void
}

// no unmap yet
#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_unmap(_addr: *mut c_void, _len: uacpi_size) {}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_alloc(size: uacpi_size) -> *mut c_void {
    unsafe {
        let layout = Layout::from_size_align(
            size + core::mem::size_of::<usize>(),
            core::mem::align_of::<usize>(),
        )
        .unwrap();
        let mem = alloc::alloc::alloc(layout);
        if mem.is_null() {
            return core::ptr::null_mut();
        }
        *(mem as *mut usize) = size;
        mem.add(core::mem::size_of::<usize>()) as *mut c_void
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_free(mem: *mut c_void) {
    unsafe {
        if !mem.is_null() {
            let real_mem = (mem as *mut u8).sub(core::mem::size_of::<usize>());
            let size = *(real_mem as *const usize);
            let layout = Layout::from_size_align(
                size + core::mem::size_of::<usize>(),
                core::mem::align_of::<usize>(),
            )
            .unwrap();
            alloc::alloc::dealloc(real_mem, layout);
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_log(lvl: uacpi_log_level, msg: *const uacpi_char) {
    if !msg.is_null() {
        let message = unsafe { CStr::from_ptr(msg).to_string_lossy().replace("\n", "") };
        match lvl {
            UACPI_LOG_DEBUG | UACPI_LOG_TRACE => debug!("{message}"),
            UACPI_LOG_INFO => info!("{message}"),
            UACPI_LOG_WARN => warn!("{message}"),
            UACPI_LOG_ERROR => error!("{message}"),
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_get_nanoseconds_since_boot() -> uacpi_u64 {
    crate::arch::drivers::time::preferred_timer_ns()
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_stall(usec: uacpi_u8) {
    let time = crate::arch::drivers::time::preferred_timer_ns();
    while crate::arch::drivers::time::preferred_timer_ns() < time + (usec as u64) * 1000 {}
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_sleep(msec: uacpi_u64) {
    let time = crate::arch::drivers::time::preferred_timer_ms();
    while time + msec > crate::arch::drivers::time::preferred_timer_ms() {}
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_create_mutex() -> uacpi_handle {
    let mutex = Box::new(Mutex::new(()));
    Box::into_raw(mutex) as *mut c_void
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_free_mutex(handle: uacpi_handle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle as *mut Mutex<()>) });
    }
}

#[derive(Default)]
struct SimpleEvent {
    counter: AtomicUsize,
}

impl SimpleEvent {
    fn decrement(&self) -> bool {
        loop {
            let value = self.counter.load(Ordering::Acquire);
            if value == 0 {
                return false;
            }
            if self
                .counter
                .compare_exchange(value, value - 1, Ordering::AcqRel, Ordering::Acquire)
                .unwrap()
                != 0
            {
                return true;
            }
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_create_event() -> uacpi_handle {
    let b = Box::new(SimpleEvent::default());
    Box::leak(b) as *mut SimpleEvent as uacpi_handle
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_free_event(handle: uacpi_handle) {
    uacpi_kernel_free(handle);
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_get_thread_id() -> uacpi_thread_id {
    1 as *mut c_void
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_acquire_mutex(handle: uacpi_handle, timeout: uacpi_u16) -> uacpi_status {
    let mutex = unsafe { &*(handle as *const Mutex<()>) };
    let mut locked = None;

    match timeout {
        0xFFFF => {
            mutex.lock();
            return UACPI_STATUS_OK;
        }
        0x0000 => {
            let time = crate::arch::drivers::time::preferred_timer_ms();
            while crate::arch::drivers::time::preferred_timer_ms() < time + timeout as u64 {
                locked = mutex.try_lock();
                if locked.is_none() {
                    uacpi_kernel_sleep(1);
                }
            }
        }
        _ => locked = mutex.try_lock(),
    }

    if locked.is_some() {
        UACPI_STATUS_OK
    } else {
        UACPI_STATUS_TIMEOUT
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_release_mutex(handle: uacpi_handle) {
    let mutex = unsafe { &*(handle as *const Mutex<()>) };
    unsafe { mutex.force_unlock() };
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_wait_for_event(handle: uacpi_handle, timeout: uacpi_u16) -> uacpi_bool {
    let event = unsafe { &mut *(handle as *mut SimpleEvent) };
    if timeout == 0xFFFF {
        while !event.decrement() {
            uacpi_kernel_sleep(10);
        }
        true
    } else {
        let mut remaining = timeout as i64;
        while !event.decrement() {
            if remaining <= 0 {
                return false;
            }
            uacpi_kernel_sleep(10);
            remaining -= 10;
        }
        true
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_signal_event(handle: uacpi_handle) {
    let event = unsafe { &mut *(handle as *mut SimpleEvent) };
    event.counter.fetch_add(1, Ordering::AcqRel);
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_reset_event(handle: uacpi_handle) {
    let event = unsafe { &mut *(handle as *mut SimpleEvent) };
    event.counter.store(0, Ordering::Release);
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_handle_firmware_request(
    _req: *mut uacpi_firmware_request,
) -> uacpi_status {
    UACPI_STATUS_OK
}

static mut UACPI_INTERRUPT_HANDLER_FN: Option<uacpi_interrupt_handler> = None;
static mut UACPI_INTERRUPT_CTX: Option<uacpi_handle> = None;

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_install_interrupt_handler(
    irq: uacpi_u32,
    func: uacpi_interrupt_handler,
    ctx: uacpi_handle,
    out_irq_handle: *mut uacpi_handle,
) -> uacpi_status {
    unsafe {
        let vector = (irq + 0x20) as u8; // x64

        UACPI_INTERRUPT_HANDLER_FN = Some(func);
        UACPI_INTERRUPT_CTX = Some(ctx);

        crate::arch::interrupts::install_interrupt(vector, handle_uacpi_interrupt);

        *(out_irq_handle as *mut usize) = vector as usize;
        UACPI_STATUS_OK
    }
}

fn handle_uacpi_interrupt(_stack_frame: *mut StackFrame) {
    unsafe {
        if let Some(handler) = UACPI_INTERRUPT_HANDLER_FN {
            handler.unwrap()(UACPI_INTERRUPT_CTX.unwrap());
        }
    }
    crate::arch::interrupts::pic::send_eoi(9);
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_uninstall_interrupt_handler(
    _func: uacpi_interrupt_handler,
    irq_handle: uacpi_handle,
) -> uacpi_status {
    unsafe {
        let vector = irq_handle as u8; // x64

        crate::arch::interrupts::clear_interrupt(vector);

        UACPI_INTERRUPT_HANDLER_FN = None;
        UACPI_INTERRUPT_CTX = None;

        UACPI_STATUS_OK
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_create_spinlock() -> uacpi_handle {
    let lock = Box::new(SpinMutex::<()>::new(()));
    Box::into_raw(lock) as *mut c_void
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_free_spinlock(handle: uacpi_handle) {
    if !handle.is_null() {
        let _ = unsafe { Box::from_raw(handle as *mut SpinMutex<()>) };
    }
}

static mut INTS_ENABLED: bool = true;

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_lock_spinlock(handle: uacpi_handle) -> uacpi_cpu_flags {
    if !handle.is_null() {
        let ints_enabled = crate::utils::asm::int_status();
        if ints_enabled {
            crate::utils::asm::toggle_ints(false);
        }
        let lock = unsafe { &*(handle as *const SpinMutex<()>) };
        lock.lock();
        unsafe { INTS_ENABLED = ints_enabled };
    }
    0
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_unlock_spinlock(handle: uacpi_handle) {
    if !handle.is_null() {
        let lock = unsafe { &*(handle as *const SpinMutex<()>) };
        unsafe { lock.force_unlock() };
        if unsafe { INTS_ENABLED } {
            crate::utils::asm::toggle_ints(true);
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_schedule_work(
    _t: uacpi_work_type,
    _handler: uacpi_work_handler,
    _ctx: uacpi_handle,
) -> uacpi_status {
    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
extern "C" fn uacpi_kernel_wait_for_work_completion() -> uacpi_status {
    UACPI_STATUS_OK
}
