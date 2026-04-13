/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::mem::ManuallyDrop;

use crate::utils::{
    asm::{int_status, toggle_ints},
    spinlock::SpinGuard,
};

use super::*;

pub struct VfsGuard {
    guard: ManuallyDrop<SpinGuard<'static, Vfs>>,
    int_status: bool,
}

impl core::ops::Deref for VfsGuard {
    type Target = Vfs;
    fn deref(&self) -> &Vfs {
        &self.guard
    }
}

impl core::ops::DerefMut for VfsGuard {
    fn deref_mut(&mut self) -> &mut Vfs {
        &mut self.guard
    }
}

impl Drop for VfsGuard {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.guard) };
        if self.int_status {
            toggle_ints(true);
        }
    }
}

pub fn get_vfs() -> VfsGuard {
    let ints_were_enabled = int_status();
    if ints_were_enabled {
        toggle_ints(false);
    }
    let guard = VFS.lock();
    VfsGuard {
        guard: ManuallyDrop::new(guard),
        int_status: ints_were_enabled,
    }
}

pub fn ls(path: Path) -> Vec<alloc::string::String> {
    get_vfs()
        .resolve_path(path)
        .unwrap()
        .get_children()
        .iter()
        .map(|node| alloc::string::String::from(node.get_name()))
        .collect()
}

pub fn cat(path: Path) -> Option<alloc::string::String> {
    get_vfs()
        .resolve_path(path)?
        .read()
        .map(|data| alloc::string::String::from(str::from_utf8(data).unwrap()))
}

pub fn rm(path: Path, name: &str) {
    get_vfs().resolve_path_mut(path).unwrap().remove_child(name);
}
