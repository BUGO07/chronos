/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    alloc::Layout,
    ffi::{CStr, c_char, c_void},
    sync::atomic::{AtomicU64, Ordering},
};
use std::{
    StackFrame,
    asm::mem::memcpy,
    fs::{Path, get_vfs},
    info,
    kernel::{
        bootloader::get_hhdm_offset,
        paging::{PAGEMAP, Table, alloc_table, flag, page_size},
        time::{get_timer, preferred_timer_ns},
    },
    memory::USER_STACK_SIZE,
    print, println,
    sched::{Process, get_proc_by_pid, get_scheduler, next_stack_address},
    spinlock::SpinLock,
    syscalls::*,
    thread::{Thread, current_thread},
};

use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use crate::arch::system::elf::load_elf;

pub fn syscall_dispatch(regs: *mut StackFrame) {
    let registers = unsafe { &mut *regs };

    let id = registers.rax;
    let arg0 = registers.rdi;
    let arg1 = registers.rsi;
    let arg2 = registers.rdx;
    // let arg3 = registers.r10;
    // let arg4 = registers.r8;
    // let arg5 = registers.r9;

    registers.rax = match id {
        READ => {
            todo!()
        }
        WRITE => {
            if arg0 == 1 {
                unsafe {
                    print!(
                        "{}",
                        core::str::from_utf8(core::slice::from_raw_parts(
                            arg1 as *const u8,
                            arg2 as usize
                        ))
                        .unwrap()
                    );
                }
            }
            0
        }
        OPEN => {
            // * complicated vfs shit
            // let ct = current_thread().as_ref().unwrap().lock();
            // let proc = ct.parent.upgrade().unwrap();
            // let mut proc = proc.lock();
            // let cwd = proc.get_cwd().clone();
            // let fd = proc.alloc_fd(Box::new(
            //     get_vfs()
            //         .resolve_path(cwd)
            //         .unwrap()
            //         .resolve_path(Path::new(unsafe {
            //             CStr::from_ptr(arg0 as *const c_char).to_str().unwrap()
            //         }))
            //         .unwrap(),
            // ));
            0
        }
        CLOSE => {
            todo!()
        }
        GETCWD => {
            let ct = current_thread().as_ref().unwrap().lock();
            let proc = ct.parent.upgrade().unwrap();
            let proc = proc.lock();
            let cwd_string = proc.get_cwd().to_string();
            if proc
                .pagemap
                .lock()
                .write_user_ptr(arg0, cwd_string.as_ptr() as u64)
                .is_ok()
            {
                0
            } else {
                1
            }
        }
        CHDIR => {
            let ct = current_thread().as_ref().unwrap().lock();
            let proc = ct.parent.upgrade().unwrap();
            let mut proc = proc.lock();
            let path =
                Path::new(unsafe { CStr::from_ptr(arg0 as *const c_char).to_str().unwrap() });
            proc.set_cwd(path);
            0
        }
        RENAME => {
            // * complicated vfs shit
            // let ct = current_thread().as_ref().unwrap().lock();
            // let proc = ct.parent.upgrade().unwrap();
            // let mut proc = proc.lock();
            // let old_path =
            //     Path::new(unsafe { CStr::from_ptr(arg0 as *const c_char).to_str().unwrap() });
            // let new_path =
            //     Path::new(unsafe { CStr::from_ptr(arg1 as *const c_char).to_str().unwrap() });
            // if let Some(node) = get_vfs()
            //     .resolve_path_mut(proc.get_cwd().clone())
            //     .unwrap()
            //     .resolve_path_mut(old_path)
            // {
            //     node.set_path(new_path);
            // }
            0
        }
        MKDIR => {
            let ct = current_thread().as_ref().unwrap().lock();
            let proc = ct.parent.upgrade().unwrap();
            let proc = proc.lock();
            let name = unsafe { CStr::from_ptr(arg0 as *const c_char).to_str().unwrap() };
            get_vfs()
                .resolve_path_mut(proc.get_cwd().clone())
                .unwrap()
                .create_dir(name)
                .unwrap();
            0
        }
        RMDIR => {
            let ct = current_thread().as_ref().unwrap().lock();
            let proc = ct.parent.upgrade().unwrap();
            let proc = proc.lock();
            let name = unsafe { CStr::from_ptr(arg0 as *const c_char).to_str().unwrap() };
            get_vfs()
                .resolve_path_mut(proc.get_cwd().clone())
                .unwrap()
                .remove_child(name);
            0
        }
        TIME => preferred_timer_ns(),
        CLOCK_GETTIME => get_timer(&arg0.into()).elapsed(),
        SHUTDOWN => {
            crate::drivers::acpi::shutdown();
            0
        }
        REBOOT => {
            crate::drivers::acpi::reboot();
            0
        }
        NANOSLEEP => {
            std::thread::sleep(arg0);
            0
        }
        SCHED_YIELD => {
            std::thread::yld();
            0
        }
        PAUSE => {
            std::thread::block();
            0
        }
        EXECVE => {
            // let path =
            //     Path::new(unsafe { CStr::from_ptr(arg0 as *const c_char).to_str().unwrap() });
            // // *filename/argv/envp
            // let mut ct = current_thread().as_ref().unwrap().lock();
            // let proc = ct.parent.upgrade().unwrap();

            // unsafe { proc.force_unlock() };
            // let proc = proc.lock();
            // // unsafe { proc.pagemap.force_unlock() };

            // // let elf = load_elf(
            // //     get_vfs()
            // //         .resolve_path_mut(path.clone())
            // //         .unwrap()
            // //         .read()
            // //         .unwrap(),
            // //     proc.pagemap.clone(),
            // // )
            // // .unwrap();
            // let elf = whattf as usize as u64;
            // info!("execve at {} - {:#X}", path, elf);
            // unsafe { proc.pagemap.force_unlock() };
            // let mut pmap = proc.pagemap.lock();
            // let arg_ptrs = pmap.read_user_array::<u64>(arg1, 256).unwrap();
            // let mut args = Vec::new();
            // for &ptr in &arg_ptrs {
            //     if ptr <= 16 {
            //         break;
            //     }
            //     let s = pmap.read_user_c_string(ptr, 256).unwrap();
            //     args.push(s);
            // }

            // let env_ptrs = pmap.read_user_array::<u64>(arg2, 256).unwrap();
            // let mut env = Vec::new();
            // for &ptr in &env_ptrs {
            //     if ptr <= 16 {
            //         break;
            //     }
            //     let s = pmap.read_user_c_string(ptr, 256).unwrap();
            //     env.push(s);
            // }
            // let mut ustack: u64 = 0;
            // let mut ustack_phys = 0;
            // let mut argv_ptrs: [u64; 64] = [0; 64];
            // let mut argc = 0;

            // ustack_phys = unsafe {
            //     alloc::alloc::alloc(
            //         Layout::from_size_align(USER_STACK_SIZE, page_size::SMALL as usize).unwrap(),
            //     ) as u64
            //         - get_hhdm_offset()
            // };

            // ustack = next_stack_address();

            // for i in (0..USER_STACK_SIZE).step_by(page_size::SMALL as usize) {
            //     pmap.map(
            //         ustack + i as u64,
            //         ustack_phys + i as u64,
            //         flag::RW | flag::USER,
            //         page_size::SMALL,
            //     );
            // }

            // let mut stack = [0u8; 0x1000];
            // let mut string_data_offset = 0x100;

            // for arg in &args {
            //     let bytes = arg.as_bytes();
            //     let len = bytes.len();

            //     if string_data_offset + len + 1 > stack.len() {
            //         panic!("Not enough stack space for argv strings");
            //     }

            //     let str_start = string_data_offset;
            //     stack[str_start..str_start + len].copy_from_slice(bytes);
            //     stack[str_start + len] = 0;

            //     argv_ptrs[argc] = ustack + str_start as u64;
            //     string_data_offset += len + 1;
            //     argc += 1;
            // }

            // argv_ptrs[argc] = 0;
            // argc += 1;

            // for (i, &ptr) in argv_ptrs[..argc].iter().enumerate() {
            //     let bytes = ptr.to_le_bytes();
            //     stack[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
            // }

            // memcpy(
            //     ustack as *mut c_void,
            //     stack.as_ptr() as *const c_void,
            //     stack.len(),
            // );

            // ct.ustack = ustack;
            // ct.ustack_phys = ustack_phys;
            // ct.regs = StackFrame {
            //     rsp: (ustack + USER_STACK_SIZE as u64) & !0xF,
            //     rip: whattf as usize as u64,
            //     rsi: ustack,
            //     rdi: argc.saturating_sub(1) as u64,
            //     rflags: 0x202,

            //     ..Default::default()
            // };

            0
        }
        FORK => {
            // let parent_thread = std::thread::current_thread().clone().unwrap();
            // let mut t_guard = parent_thread.lock();
            // let parent_proc = t_guard.get_parent().upgrade().unwrap();
            // let p_guard = parent_proc.lock();

            // let scheduler = get_scheduler();
            // let new_pid = scheduler.next_pid.fetch_add(1, Ordering::Relaxed);
            // let child_proc = Arc::new(SpinLock::new(Process {
            //     name: p_guard.name,
            //     pid: new_pid,
            //     cwd: p_guard.cwd.clone(),
            //     next_tid: AtomicU64::new(1),
            //     children: Vec::new(),
            //     fd_table: Vec::new(),
            //     pagemap: p_guard.pagemap.clone(),
            // }));

            // scheduler.processes.push(child_proc.clone());

            // let new_thread = Arc::new(SpinLock::new(Thread::new(
            //     get_proc_by_pid(new_pid).unwrap(),
            //     t_guard.entry as usize,
            //     t_guard.name,
            //     t_guard.is_user(),
            //     t_guard.args.clone(),
            // )));

            // {
            //     let mut new_t_guard = new_thread.lock();
            //     new_t_guard.regs = t_guard.regs;
            //     let prip = t_guard.regs.rip;
            //     info!("Parent rip before fork: {:#x}", prip);
            //     new_t_guard.regs.rip = prip - 26;
            //     let chrip = new_t_guard.regs.rip;
            //     info!("Child rip after increment: {:#x}", chrip);
            //     new_t_guard.regs.rax = 0;
            // }

            // {
            //     // Push thread into the child_proc BEFORE enqueue
            //     let mut child_guard = child_proc.lock();
            //     child_guard.children.push(new_thread.clone());
            // }

            // // Enqueue after unlocking everything else
            // std::sched::enqueue(new_thread.clone());

            // // Set return value for parent
            // t_guard.regs.rax = new_pid as u64;

            // let hhdm = get_hhdm_offset();
            // let stack_page = t_guard.regs.rsp & !0xfff;
            // let sp = t_guard.regs.rsp;
            // info!("fork: rsp = {:#x}, aligned to {:#x}", sp, stack_page);

            // // Assume you have cloned these outside:
            // let parent_pagemap = parent_proc.lock().pagemap.clone();
            // let child_pagemap = child_proc.lock().pagemap.clone();

            // for i in 0..4 {
            //     let src_vaddr = stack_page + i * 0x1000;
            //     info!("fork: copying page at vaddr {:#x}", src_vaddr);

            //     let parent_guard = parent_pagemap.lock();
            //     let Some(phys) = parent_guard.translate(src_vaddr) else {
            //         info!(" -> page not mapped in parent; skipping");
            //         continue;
            //     };
            //     drop(parent_guard);
            //     info!(" -> parent phys: {:#x}", phys);

            //     let new_phys = alloc_table() as u64;
            //     let src_ptr = (phys + hhdm) as *const u8;
            //     let dst_ptr = (new_phys + hhdm) as *mut u8;

            //     unsafe {
            //         memcpy(dst_ptr as *mut c_void, src_ptr as *const c_void, 4096);
            //         info!(" -> copied 4KiB from {:#x} to {:#x}", phys, new_phys);

            //         // Print first 4 bytes as sanity check
            //         info!(
            //             " -> bytes: {:02x} {:02x} {:02x} {:02x}",
            //             *dst_ptr.offset(0),
            //             *dst_ptr.offset(1),
            //             *dst_ptr.offset(2),
            //             *dst_ptr.offset(3)
            //         );
            //     }

            //     let mut child_guard = child_pagemap.lock();
            //     match child_guard.map(src_vaddr, new_phys, flag::RW | flag::USER, page_size::SMALL)
            //     {
            //         true => info!(
            //             " -> mapped {:#x} into child pagemap at {:#x}",
            //             new_phys, src_vaddr
            //         ),
            //         false => panic!(" -> map failed for {:#x}", src_vaddr),
            //     }
            //     drop(child_guard);
            // }

            // info!("fork() done, parent returns child pid {}", new_pid);

            // new_pid

            0
        }

        303 => std::sched::spawn_process(
            str::from_utf8(unsafe {
                core::slice::from_raw_parts(arg0 as *const u8, arg1 as usize)
            })
            .unwrap_or("unknown"),
        ),
        GETTID => {
            if let Some(thread) = std::thread::current_thread() {
                thread.lock().get_tid()
            } else {
                0
            }
        }
        EXIT => {
            // error code arg0 unused
            if let Some(thread) = std::thread::current_thread() {
                thread
                    .lock()
                    .set_status(std::thread::ThreadStatus::Terminated);
                std::thread::yld();
                0
            } else {
                1
            }
        }
        KILL => {
            let pid = arg0;
            // let sig = arg1;
            std::sched::kill_process(pid) as u64
        }
        GETPID => {
            if let Some(thread) = std::thread::current_thread() {
                thread
                    .lock()
                    .parent
                    .upgrade()
                    .clone()
                    .unwrap()
                    .lock()
                    .get_pid()
            } else {
                0
            }
        }
        // 307 => {
        //     if arg0 == 0 || arg1 == 0 {
        //         0
        //     } else {
        //         let string = copy_user_string(arg0, arg1 as usize).unwrap();
        //         let path = Path::new("/bin").join(string.as_str());
        //         let file = unsafe {
        //             VFS.get()
        //                 .unwrap()
        //                 .resolve_path(path)
        //                 .unwrap()
        //                 .read()
        //                 .unwrap()
        //                 .as_slice()
        //         };
        //         let elf =
        //             load_elf(file, get_proc_by_pid(arg2).unwrap().lock().pagemap.clone()).unwrap();
        //         info!("elf - {elf:#x}");
        //         elf
        //     }
        // }
        _ => {
            print!("Unknown syscall {}\n", id);
            0
        }
    }
}

// #[unsafe(no_mangle)]
// extern "C" fn whattf(argc: u64, argv: *const *const c_char) -> ! {
//     let s = format!("{argc} - {:#x}\n", argv as u64);
//     _syscall(WRITE, 1, s.as_ptr() as u64, s.len() as u64, 0, 0, 0);

//     let s = if argc != 0 {
//         unsafe {
//             core::slice::from_raw_parts(argv, argc as usize)
//                 .iter()
//                 .map(|x| CStr::from_ptr(*x).to_str().unwrap())
//                 .collect::<Vec<_>>()
//         }
//         .join(" ")
//     } else {
//         "shice".to_string()
//     } + "\n";
//     _syscall(WRITE, 1, s.as_ptr() as u64, s.len() as u64, 0, 0, 0);
//     loop {}
// }

pub fn _syscall(id: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64 {
    let ret: i64;
    unsafe {
        core::arch::asm!(
            "int $0x80",
            in("rax") id,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            in("r10") arg3,
            in("r8") arg4,
            in("r9") arg5,
            lateout("rax") ret,
            options(nomem, nostack, preserves_flags)
        );
    }
    ret
}
