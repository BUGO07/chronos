/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::cell::OnceCell;

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::{debug, info};

pub use types::*;
pub mod helpers;
pub mod types;
pub use helpers::*;

pub static mut VFS: OnceCell<Vfs> = OnceCell::new();

impl Vfs {
    pub fn new(root: Box<dyn VfsNode>) -> Self {
        Self { root }
    }
    pub fn get_root(&self) -> &dyn VfsNode {
        self.root.as_ref()
    }
    pub fn get_root_mut(&mut self) -> &mut Box<dyn VfsNode> {
        &mut self.root
    }
    pub fn resolve_path(&self, path: Path) -> Option<&dyn VfsNode> {
        self.root.resolve_path(path)
    }
    pub fn resolve_path_mut(&mut self, path: Path) -> Option<&mut dyn VfsNode> {
        self.root.resolve_path_mut(path)
    }
}
pub trait VfsNode: core::fmt::Debug {
    fn get_permissions(&self) -> &Permissions;
    fn get_permissions_mut(&mut self) -> &mut Permissions;
    fn get_metadata(&self) -> &VfsNodeMetadata;
    fn get_metadata_mut(&mut self) -> &mut VfsNodeMetadata;
    fn get_type(&self) -> &VfsNodeType;
    fn is_dir(&self) -> bool {
        self.get_type() == &VfsNodeType::Directory
    }
    fn is_file(&self) -> bool {
        self.get_type() == &VfsNodeType::File
    }
    fn get_path(&self) -> &Path;
    fn get_name(&self) -> &'_ str;
    fn get_parent(&self) -> Option<&dyn VfsNode> {
        get_vfs().resolve_path(self.get_path().get_parent())
    }
    fn get_parent_mut(&mut self) -> Option<&mut dyn VfsNode> {
        get_vfs().resolve_path_mut(self.get_path().get_parent())
    }
    fn get_child(&self, name: &str) -> Option<&dyn VfsNode>; // folder-only
    fn get_child_mut(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>>; // folder-only
    fn get_children(&self) -> Vec<&dyn VfsNode>; // folder-only
    fn get_children_mut(&mut self) -> &mut Vec<Box<dyn VfsNode>>; // folder-only
    fn resolve_path(&self, path: Path) -> Option<&dyn VfsNode>; // folder-only
    fn resolve_path_mut(&mut self, path: Path) -> Option<&mut dyn VfsNode>; // folder-only
    fn create_dir(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>>; // folder-only
    fn create_file(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>>; // folder-only
    fn remove_child(&mut self, name: &str); // folder-only
    fn read(&self) -> Option<&Vec<u8>>; // file-only
    fn write(&mut self, data: Vec<u8>); // file-only
}

impl VfsNode for Directory {
    fn get_permissions(&self) -> &Permissions {
        &self.get_metadata().permissions
    }
    fn get_permissions_mut(&mut self) -> &mut Permissions {
        &mut self.get_metadata_mut().permissions
    }
    fn get_metadata(&self) -> &VfsNodeMetadata {
        &self.metadata
    }
    fn get_metadata_mut(&mut self) -> &mut VfsNodeMetadata {
        &mut self.metadata
    }
    fn get_type(&self) -> &VfsNodeType {
        &self.get_metadata().r#type
    }
    fn get_path(&self) -> &Path {
        &self.path
    }
    fn get_name(&self) -> &'_ str {
        self.path.get_name()
    }
    fn resolve_path(&self, path: Path) -> Option<&dyn VfsNode> {
        let mut current = self as &dyn VfsNode;
        let string = path.to_string();
        if string == "/" {
            return Some(get_vfs().get_root());
        }
        for part in string.split("/") {
            if part.is_empty() || part == "." {
                continue;
            } else if part == ".." {
                if let Some(parent) = current.get_parent() {
                    current = parent;
                } else {
                    return None;
                }
            } else if let Some(child) = current.get_child(part) {
                current = child;
            } else {
                return None;
            }
        }
        Some(current)
    }
    fn resolve_path_mut(&mut self, path: Path) -> Option<&mut dyn VfsNode> {
        let mut current = self as &mut dyn VfsNode;

        let string = path.to_string();
        if string == "/" {
            return Some(get_vfs().get_root_mut().as_mut());
        }
        for part in string.split("/") {
            if part.is_empty() || part == "." {
                continue;
            } else if part == ".." {
                if let Some(parent) = current.get_parent_mut() {
                    current = parent;
                } else {
                    return None;
                }
            } else if let Some(child) = current.get_child_mut(part) {
                current = child.as_mut();
            } else {
                return None;
            }
        }
        Some(current)
    }
    fn get_child(&self, name: &str) -> Option<&dyn VfsNode> {
        self.children
            .iter()
            .map(|c| c.as_ref())
            .find(|c| c.get_name() == name)
    }
    fn get_child_mut(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>> {
        self.children.iter_mut().find(|c| c.get_name() == name)
    }
    fn get_children(&self) -> Vec<&dyn VfsNode> {
        self.children.iter().map(|c| c.as_ref()).collect()
    }
    fn get_children_mut(&mut self) -> &mut Vec<Box<dyn VfsNode>> {
        &mut self.children
    }
    fn create_dir(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>> {
        self.children
            .push(Box::new(Directory::new(self.path.join(name))));
        self.children.last_mut()
    }
    fn create_file(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>> {
        self.children
            .push(Box::new(File::new(Vec::new(), self.path.join(name))));
        self.children.last_mut()
    }
    fn remove_child(&mut self, name: &str) {
        self.children.retain(|c| c.get_name() != name);
    }
    fn read(&self) -> Option<&Vec<u8>> {
        None
    }
    fn write(&mut self, _data: Vec<u8>) {}
}

impl VfsNode for File {
    fn get_permissions(&self) -> &Permissions {
        &self.get_metadata().permissions
    }
    fn get_permissions_mut(&mut self) -> &mut Permissions {
        &mut self.get_metadata_mut().permissions
    }
    fn get_metadata(&self) -> &VfsNodeMetadata {
        &self.metadata
    }
    fn get_metadata_mut(&mut self) -> &mut VfsNodeMetadata {
        &mut self.metadata
    }
    fn get_type(&self) -> &VfsNodeType {
        &self.get_metadata().r#type
    }
    fn get_path(&self) -> &Path {
        &self.path
    }
    fn get_name(&self) -> &'_ str {
        self.path.get_name()
    }
    fn resolve_path(&self, _path: Path) -> Option<&dyn VfsNode> {
        None
    }
    fn resolve_path_mut(&mut self, _path: Path) -> Option<&mut dyn VfsNode> {
        None
    }
    fn get_child(&self, _name: &str) -> Option<&dyn VfsNode> {
        None
    }
    fn get_child_mut(&mut self, _name: &str) -> Option<&mut Box<dyn VfsNode>> {
        None
    }
    fn get_children(&self) -> Vec<&dyn VfsNode> {
        Vec::new()
    }
    fn get_children_mut(&mut self) -> &mut Vec<Box<dyn VfsNode>> {
        static mut EMPTY_DATA: Vec<Box<dyn VfsNode>> = Vec::new(); // holy bad code
        unsafe { &mut EMPTY_DATA }
    }
    fn create_dir(&mut self, _name: &str) -> Option<&mut Box<dyn VfsNode>> {
        None
    }
    fn create_file(&mut self, _name: &str) -> Option<&mut Box<dyn VfsNode>> {
        None
    }
    fn remove_child(&mut self, _name: &str) {}
    fn read(&self) -> Option<&Vec<u8>> {
        Some(&self.data)
    }
    fn write(&mut self, data: Vec<u8>) {
        self.metadata.size = data.len() as u64;
        self.data = data;
    }
}

impl Path {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }
    pub fn set(&mut self, path: String) {
        self.path = path;
    }
    pub fn join(&mut self, path: &str) -> Self {
        let formatted = if self.is_root() {
            format!("/{path}")
        } else {
            format!("{}/{}", self.path, path)
        };
        Self { path: formatted }
    }
    pub fn get_parent(&self) -> Self {
        Self {
            path: self
                .path
                .split("/")
                .take(self.path.split("/").count() - 1)
                .collect::<Vec<_>>()
                .join("/"),
        }
    }
    pub fn get_name(&self) -> &'_ str {
        self.path.split("/").last().unwrap_or(&self.path)
    }
    pub fn to_string(&self) -> &String {
        &self.path
    }
    pub fn is_root(&self) -> bool {
        self.path == "/"
    }
}

pub fn init() {
    info!("initializing vfs...");
    unsafe {
        let mut vfs = Vfs::new(Box::new(Directory::new(Path::new("/"))));
        let root = vfs.get_root_mut();
        debug!("creating /home...");
        let home = root.create_dir("home").unwrap();
        debug!("creating /home/secrets.txt...");
        let file = home.create_file("secrets.txt").unwrap();
        debug!("writing to {}...", file.get_path());
        file.write("secretpassword".as_bytes().to_vec());
        debug!(
            "reading from {}...\n{:?}",
            file.get_path(),
            str::from_utf8(file.read().unwrap()).unwrap()
        );
        VFS.set(vfs).ok();
        let file = get_vfs()
            .get_root_mut()
            .get_child_mut("home")
            .unwrap()
            .get_child_mut("secrets.txt")
            .unwrap();
        debug!("editing {}...", file.get_path());
        file.write("newsecretpassword".as_bytes().to_vec());
        debug!(
            "re-reading from {}...\n{:?}",
            file.get_path(),
            str::from_utf8(file.read().unwrap()).unwrap()
        );
    }
    info!("done");
}
