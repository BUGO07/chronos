/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use super::*;

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
