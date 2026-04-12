use crate::drivers::fs::VfsNodeType;

pub struct TarHeader<'a> {
    pub name: &'a str,
    pub type_: VfsNodeType,
    pub start: usize,
    pub end: usize,
}

fn oct2bin(s: &[u8]) -> usize {
    let mut n = 0;
    for &c in s {
        if (b'0'..=b'7').contains(&c) {
            n = n * 8 + (c - b'0') as usize;
        }
    }
    n
}

fn parse_header<'a>(archive: &'a [u8], offset: usize) -> Option<TarHeader<'a>> {
    if offset + 512 > archive.len() {
        return None;
    }

    let header = &archive[offset..offset + 512];
    if &header[257..257 + 5] != b"ustar" {
        return None;
    }

    let type_ = match header[156] {
        b'0' => VfsNodeType::File,
        b'5' => VfsNodeType::Directory,
        _ => return None,
    };

    let size = oct2bin(&header[0x7c..0x7c + 11]);
    let start = offset + 512;
    let end = start + size;
    if end > archive.len() {
        return None;
    }

    let raw_name = &header[..100];
    let nul_pos = raw_name
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(raw_name.len());
    let name = core::str::from_utf8(&raw_name[..nul_pos]).ok()?;

    Some(TarHeader {
        name,
        type_,
        start,
        end,
    })
}

pub struct TarIter<'a> {
    archive: &'a [u8],
    offset: usize,
}

impl<'a> TarIter<'a> {
    pub fn new(archive: &'a [u8]) -> Self {
        Self { archive, offset: 0 }
    }
}

impl<'a> Iterator for TarIter<'a> {
    type Item = TarHeader<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let header = parse_header(self.archive, self.offset)?;
        // move to next header
        let size = header.end - header.start;
        let blocks = size.div_ceil(512);
        self.offset += 512 + blocks * 512;
        Some(header)
    }
}

pub fn tar_lookup<'a>(archive: &'a [u8], filename: &str) -> Option<&'a [u8]> {
    for header in TarIter::new(archive) {
        if header.name == filename && header.end <= archive.len() {
            return Some(&archive[header.start..header.end]);
        }
    }
    None
}
