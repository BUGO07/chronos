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

#[derive(Debug)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Permissions {
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }
}

#[derive(Debug)]
pub struct VfsNodeMetadata {
    pub size: u64,
    pub created_at: u64,
    pub modified_at: u64,
    pub r#type: VfsNodeType,
    pub permissions: Permissions,
}

impl VfsNodeMetadata {
    pub fn new(r#type: VfsNodeType) -> Self {
        VfsNodeMetadata {
            size: 0,
            created_at: 0,
            modified_at: 0,
            r#type,
            permissions: Permissions::new(true, true, false),
        }
    }

    pub fn with_permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn with_created_at(mut self, created_at: u64) -> Self {
        self.created_at = created_at;
        self
    }

    pub fn with_modified_at(mut self, modified_at: u64) -> Self {
        self.modified_at = modified_at;
        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VfsNodeType {
    File,
    Directory,
}

#[derive(Debug)]
pub struct Vfs {
    pub root: Box<dyn VfsNode>,
}

#[derive(Debug, Clone)]
pub struct Path {
    pub path: String,
}

impl core::fmt::Display for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.path)
    }
}

pub struct Directory {
    pub children: Vec<Box<dyn VfsNode>>,
    pub metadata: VfsNodeMetadata,
    pub path: Path,
}

impl core::fmt::Debug for Directory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?}, {:?}, {:?}",
            self.path, self.children, self.metadata
        )
    }
}

impl Directory {
    pub fn new(path: Path) -> Self {
        Self {
            children: Vec::new(),
            metadata: VfsNodeMetadata::new(VfsNodeType::Directory),
            path,
        }
    }
}

#[derive(Debug)]
pub struct File {
    pub data: Vec<u8>,
    pub metadata: VfsNodeMetadata,
    pub path: Path,
}

impl File {
    pub fn new(data: Vec<u8>, path: Path) -> Self {
        Self {
            data,
            metadata: VfsNodeMetadata::new(VfsNodeType::File),
            path,
        }
    }
}

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
