/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::boxed::Box;

use std::{
    debug,
    fs::{Directory, Path, VFS, Vfs},
    info,
};

pub fn init() {
    info!("initializing vfs...");
    unsafe {
        let mut vfs = Vfs::new(Box::new(Directory::new(Path::new("/"))));
        let root = vfs.get_root_mut();
        debug!("creating /home...");
        root.create_dir("home").unwrap();
        debug!("writing binaries to /bin");
        let bin = root.create_dir("bin").unwrap();
        let echo = bin.create_file("echo").unwrap();
        echo.write(include_bytes!("../../../../userspace/elfs/echo").to_vec());
        echo.get_permissions_mut().execute = true;
        let shell = bin.create_file("shell").unwrap();
        shell.write(include_bytes!("../../../../userspace/elfs/shell").to_vec());
        shell.get_permissions_mut().execute = true;
        VFS.set(vfs).ok();
    }
    info!("done");
}
