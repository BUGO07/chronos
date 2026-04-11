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

fn canonicalize_components(path: &Path) -> Option<Vec<&str>> {
    let mut out: Vec<&str> = Vec::new();
    for part in path.as_str().split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            out.pop()?;
            continue;
        }
        out.push(part);
    }
    Some(out)
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
    fn get_child(&self, name: &str) -> Option<&dyn VfsNode>; // folder-only
    fn get_child_mut(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>>; // folder-only
    fn get_children(&self) -> Vec<&dyn VfsNode>; // folder-only
    fn resolve_path(&self, path: Path) -> Option<&dyn VfsNode>; // folder-only
    fn resolve_path_mut(&mut self, path: Path) -> Option<&mut dyn VfsNode>; // folder-only
    fn create_dir(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>>; // folder-only
    fn create_file(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>>; // folder-only
    fn remove_child(&mut self, name: &str) -> bool; // folder-only

    // File-only ops. Directories implement these as no-ops / None.
    fn size(&self) -> u64;
    fn read(&self) -> Option<&[u8]>;
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Option<usize>;
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Option<usize>;
    fn truncate(&mut self, len: u64) -> bool;
    fn write_all(&mut self, data: &[u8]) -> bool {
        if !self.truncate(0) {
            return false;
        }
        if data.is_empty() {
            return true;
        }
        self.write_at(0, data).is_some()
    }
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
        let components = canonicalize_components(&path)?;
        let mut current: &dyn VfsNode = self;
        for part in components {
            current = current.get_child(part)?;
        }
        Some(current)
    }
    fn resolve_path_mut(&mut self, path: Path) -> Option<&mut dyn VfsNode> {
        let components = canonicalize_components(&path)?;
        let mut current: &mut dyn VfsNode = self;
        for part in components {
            let child = current.get_child_mut(part)?;
            current = child.as_mut();
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
    fn create_dir(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>> {
        if self.children.iter().any(|c| c.get_name() == name) {
            return None;
        }
        self.children
            .push(Box::new(Directory::new(self.path.join(name))));
        self.children.last_mut()
    }
    fn create_file(&mut self, name: &str) -> Option<&mut Box<dyn VfsNode>> {
        if self.children.iter().any(|c| c.get_name() == name) {
            return None;
        }
        self.children
            .push(Box::new(File::new(Vec::new(), self.path.join(name))));
        self.children.last_mut()
    }
    fn remove_child(&mut self, name: &str) -> bool {
        let before = self.children.len();
        self.children.retain(|c| c.get_name() != name);
        self.children.len() != before
    }
    fn size(&self) -> u64 {
        0
    }
    fn read(&self) -> Option<&[u8]> {
        None
    }
    fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> Option<usize> {
        None
    }
    fn write_at(&mut self, _offset: u64, _buf: &[u8]) -> Option<usize> {
        None
    }
    fn truncate(&mut self, _len: u64) -> bool {
        false
    }
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
    fn resolve_path(&self, path: Path) -> Option<&dyn VfsNode> {
        let components = canonicalize_components(&path)?;
        if components.is_empty() {
            Some(self)
        } else {
            None
        }
    }
    fn resolve_path_mut(&mut self, path: Path) -> Option<&mut dyn VfsNode> {
        let components = canonicalize_components(&path)?;
        if components.is_empty() {
            Some(self)
        } else {
            None
        }
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
    fn create_dir(&mut self, _name: &str) -> Option<&mut Box<dyn VfsNode>> {
        None
    }
    fn create_file(&mut self, _name: &str) -> Option<&mut Box<dyn VfsNode>> {
        None
    }
    fn remove_child(&mut self, _name: &str) -> bool {
        false
    }
    fn size(&self) -> u64 {
        self.data.len() as u64
    }
    fn read(&self) -> Option<&[u8]> {
        Some(&self.data)
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Option<usize> {
        let offset = offset as usize;
        if offset > self.data.len() {
            return None;
        }
        let available = self.data.len() - offset;
        let n = available.min(buf.len());
        buf[..n].copy_from_slice(&self.data[offset..offset + n]);
        Some(n)
    }
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Option<usize> {
        let offset = offset as usize;
        let end = offset.checked_add(buf.len())?;
        if self.data.len() < end {
            self.data.resize(end, 0);
        }
        self.data[offset..end].copy_from_slice(buf);
        self.metadata.size = self.data.len() as u64;
        Some(buf.len())
    }
    fn truncate(&mut self, len: u64) -> bool {
        let len = len as usize;
        if len <= self.data.len() {
            self.data.truncate(len);
        } else {
            self.data.resize(len, 0);
        }
        self.metadata.size = self.data.len() as u64;
        true
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
    pub fn join(&self, path: &str) -> Self {
        let formatted = if self.is_root() {
            format!("/{path}")
        } else {
            format!("{}/{}", self.path, path)
        };
        Self { path: formatted }
    }
    pub fn get_parent(&self) -> Self {
        if self.is_root() {
            return Self::new("/");
        }
        match self.path.rfind('/') {
            Some(0) => Self::new("/"),
            Some(idx) => Self {
                path: self.path[..idx].to_string(),
            },
            None => Self::new("/"),
        }
    }
    pub fn get_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
    pub fn as_str(&self) -> &str {
        &self.path
    }
    pub fn is_root(&self) -> bool {
        self.path == "/"
    }
    pub fn is_absolute(&self) -> bool {
        self.path.starts_with('/')
    }
}

pub fn init() {
    info!("initializing vfs...");
    let mut vfs = Vfs::new(Box::new(Directory::new(Path::new("/"))));

    for module in crate::utils::limine::get_modules() {
        let tar = module.data();
        for item in crate::utils::ustar::TarIter::new(tar) {
            if let Some(data) = crate::utils::ustar::tar_lookup(tar, item.name) {
                debug!(
                    "creating {} - {}",
                    if item.name.ends_with("/") {
                        "folder"
                    } else {
                        "file"
                    },
                    &item.name[1..]
                );
                if item.name.ends_with("/") {
                    if &item.name[1..] == "/" {
                        continue;
                    }
                    let path = Path::new(&item.name[1..item.name.len() - 1]);

                    if let Some(parent) = vfs.resolve_path_mut(path.get_parent()) {
                        parent.create_dir(path.get_name()).unwrap();
                    }
                } else {
                    let path = Path::new(&item.name[1..]);

                    if let Some(parent) = vfs.resolve_path_mut(path.get_parent())
                        && let Some(file) = parent.create_file(path.get_name())
                    {
                        file.write_all(data);
                    }
                }
            }
        }
    }

    unsafe { VFS.set(vfs).ok() };

    info!("done");
}
