/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::arch::drivers::time::rtc::read_rtc;

use super::*;

bitflags::bitflags! {
    #[derive(Debug)]
    pub struct NodeMode: i32 {
        const S_IRUSR = 0o400;
        const S_IWUSR = 0o200;
        const S_IXUSR = 0o100;
        const S_IRGRP = 0o040;
        const S_IWGRP = 0o020;
        const S_IXGRP = 0o010;
        const S_IROTH = 0o004;
        const S_IWOTH = 0o002;
        const S_IXOTH = 0o001;
        const READ = Self::S_IRUSR.bits() | Self::S_IRGRP.bits() | Self::S_IROTH.bits();
        const WRITE = Self::S_IWUSR.bits() | Self::S_IWGRP.bits() | Self::S_IWOTH.bits();
        const EXECUTE = Self::S_IXUSR.bits() | Self::S_IXGRP.bits() | Self::S_IXOTH.bits();
        const RW = Self::READ.bits() | Self::WRITE.bits();
    }

    #[derive(PartialEq, Debug, Clone, Copy)]
    pub struct Permissions: i32 {
        const READ = 0b100;
        const WRITE = 0b010;
        const EXECUTE = 0b001;
        const RW = Self::READ.bits() | Self::WRITE.bits();
        const RWX = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }
}

#[derive(Debug)]
pub struct VfsNodeMetadata {
    pub size: u64,
    pub created_at: u64,
    pub modified_at: u64,
    pub type_: VfsNodeType,
    pub permissions: NodeMode,
}

impl VfsNodeMetadata {
    pub fn new(type_: VfsNodeType) -> Self {
        VfsNodeMetadata {
            size: 0,
            created_at: 0,
            modified_at: 0,
            type_,
            permissions: NodeMode::RW,
        }
    }

    pub fn with_permissions(mut self, permissions: NodeMode) -> Self {
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
    pub root: Option<Box<dyn VfsNode>>,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    InvalidPath,
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
        let epoch = read_rtc().to_epoch().unwrap_or_default();
        Self {
            children: Vec::new(),
            metadata: VfsNodeMetadata::new(VfsNodeType::Directory)
                .with_created_at(epoch)
                .with_modified_at(epoch),
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
        let epoch = read_rtc().to_epoch().unwrap_or_default();
        let size = data.len() as u64;
        Self {
            data,
            metadata: VfsNodeMetadata::new(VfsNodeType::File)
                .with_created_at(epoch)
                .with_modified_at(epoch)
                .with_size(size),
            path,
        }
    }
}
