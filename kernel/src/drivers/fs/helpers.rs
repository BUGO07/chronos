/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use super::*;

pub fn get_root() -> &'static dyn VfsNode {
    get_vfs().get_root()
}

pub fn get_root_mut() -> &'static mut Box<dyn VfsNode> {
    get_vfs().get_root_mut()
}

pub fn get_vfs() -> &'static mut Vfs {
    unsafe { VFS.get_mut().unwrap() }
}

pub fn ls(path: Path) -> Vec<&'static str> {
    get_vfs()
        .resolve_path(path)
        .unwrap()
        .get_children()
        .iter()
        .map(|node| node.get_name())
        .collect()
}

pub fn cat(path: Path) -> Option<&'static str> {
    get_vfs()
        .resolve_path(path)
        .unwrap()
        .read()
        .map(|data| str::from_utf8(data).unwrap())
}

pub fn mkfile(path: Path, name: &str) {
    get_vfs()
        .resolve_path_mut(path)
        .unwrap()
        .create_file(name)
        .unwrap();
}

pub fn mkdir(path: Path, name: &str) {
    get_vfs()
        .resolve_path_mut(path)
        .unwrap()
        .create_dir(name)
        .unwrap();
}

pub fn rm(path: Path, name: &str) {
    get_vfs().resolve_path_mut(path).unwrap().remove_child(name);
}
