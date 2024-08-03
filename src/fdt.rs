use core::ffi::CStr;

use crate::util::align_up;

#[allow(unused)]
struct FdtHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

impl FdtHeader {
    unsafe fn from_ptr(ptr: *const u8) -> Option<Self> {
        let ptr: *const u32 = ptr.cast();

        let magic = u32::from_be(ptr.add(0).read());
        if magic != 0xd00dfeed {
            return None;
        }

        let totalsize = u32::from_be(ptr.add(1).read());
        let off_dt_struct = u32::from_be(ptr.add(2).read());
        let off_dt_strings = u32::from_be(ptr.add(3).read());
        let off_mem_rsvmap = u32::from_be(ptr.add(4).read());

        let version = u32::from_be(ptr.add(5).read());
        let last_comp_version = u32::from_be(ptr.add(6).read());
        if version < 17 || last_comp_version > 17 {
            return None;
        }

        let boot_cpuid_phys = u32::from_be(ptr.add(7).read());
        let size_dt_strings = u32::from_be(ptr.add(8).read());
        let size_dt_struct = u32::from_be(ptr.add(9).read());

        Some(Self {
            magic,
            totalsize,
            off_dt_struct,
            off_dt_strings,
            off_mem_rsvmap,
            version,
            last_comp_version,
            boot_cpuid_phys,
            size_dt_strings,
            size_dt_struct,
        })
    }
}

pub struct Fdt<'a> {
    header: FdtHeader,
    data: &'a [u8],
}

impl<'a> Fdt<'a> {
    pub unsafe fn from_ptr(ptr: *const u8) -> Option<Self> {
        let header = FdtHeader::from_ptr(ptr)?;
        let data = core::slice::from_raw_parts(ptr, header.totalsize as usize);
        Some(Self { header, data })
    }

    pub fn memory_reservations(&self) -> impl Iterator<Item = MemoryReservation> + 'a {
        let memresv = &self.data[self.header.off_mem_rsvmap as usize..];
        memresv.chunks_exact(16).map_while(|chunk| {
            let address = u64::from_be_bytes(chunk[0..8].try_into().unwrap());
            let size = u64::from_be_bytes(chunk[8..16].try_into().unwrap());
            if address == 0 && size == 0 {
                None
            } else {
                Some(MemoryReservation { address, size })
            }
        })
    }

    pub fn root_node(&self) -> FdtNode<'a> {
        let mut iter = self.struct_items();
        match iter.next() {
            Some(StructItem::BeginNode { name }) => FdtNode { name, iter },
            _ => panic!("expected FDT_BEGIN_NODE"),
        }
    }

    fn struct_items(&self) -> StructItemIter<'a> {
        let dt_struct =
            &self.data[self.header.off_dt_struct as usize..][..self.header.size_dt_struct as usize];
        let dt_strings = &self.data[self.header.off_dt_strings as usize..]
            [..self.header.size_dt_strings as usize];
        StructItemIter {
            dt_struct,
            dt_strings,
        }
    }
}

pub struct MemoryReservation {
    pub address: u64,
    pub size: u64,
}

pub struct FdtNode<'a> {
    pub name: &'a str,
    iter: StructItemIter<'a>,
}

impl<'a> FdtNode<'a> {
    pub fn properties(&self) -> impl Iterator<Item = Property<'a>> {
        self.iter
            .clone()
            .map_while(|item| match item {
                StructItem::Prop { name, value } => Some(Property { name, value }),
                _ => None,
            })
            .fuse()
    }

    pub fn children(&self) -> Children<'a> {
        Children {
            iter: self.iter.clone(),
            depth: 1,
        }
    }
}

pub struct Property<'a> {
    pub name: &'a str,
    pub value: &'a [u8],
}

pub struct Children<'a> {
    iter: StructItemIter<'a>,
    depth: usize,
}

impl<'a> Iterator for Children<'a> {
    type Item = FdtNode<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        while self.depth > 0 {
            match self.iter.next()? {
                StructItem::BeginNode { name } => {
                    self.depth += 1;
                    if self.depth == 2 {
                        return Some(FdtNode {
                            name,
                            iter: self.iter.clone(),
                        });
                    }
                }
                StructItem::EndNode => self.depth -= 1,
                StructItem::Prop { .. } => {}
            }
        }
        None
    }
}

enum StructItem<'a> {
    BeginNode { name: &'a str },
    EndNode,
    Prop { name: &'a str, value: &'a [u8] },
}

#[derive(Clone, Copy)]
struct StructItemIter<'a> {
    dt_struct: &'a [u8],
    dt_strings: &'a [u8],
}

impl<'a> Iterator for StructItemIter<'a> {
    type Item = StructItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.dt_struct.is_empty() {
            let node = u32::from_be_bytes(self.dt_struct[0..4].try_into().unwrap());
            match node {
                FDT_BEGIN_NODE => {
                    let name = CStr::from_bytes_until_nul(&self.dt_struct[4..]).unwrap();
                    let name = name.to_str().unwrap();
                    let sz_name = align_up(name.len() + 1, 4);
                    self.dt_struct = &self.dt_struct[4 + sz_name..];
                    return Some(StructItem::BeginNode { name });
                }
                FDT_END_NODE => {
                    self.dt_struct = &self.dt_struct[4..];
                    return Some(StructItem::EndNode);
                }
                FDT_PROP => {
                    let len = u32::from_be_bytes(self.dt_struct[4..8].try_into().unwrap());
                    let nameoff = u32::from_be_bytes(self.dt_struct[8..12].try_into().unwrap());
                    let name =
                        CStr::from_bytes_until_nul(&self.dt_strings[nameoff as usize..]).unwrap();
                    let name = name.to_str().unwrap();
                    let value = &self.dt_struct[12..][..len as usize];

                    let aligned = align_up(len as usize, 4);
                    self.dt_struct = &self.dt_struct[12 + aligned..];
                    return Some(StructItem::Prop { name, value });
                }
                FDT_NOP => self.dt_struct = &self.dt_struct[4..],
                FDT_END => {
                    self.dt_struct = &self.dt_struct[4..];
                    debug_assert!(self.dt_struct.is_empty());
                }
                _ => panic!("unrecognized FDT node"),
            }
        }
        None
    }
}

const FDT_BEGIN_NODE: u32 = 0x1;
const FDT_END_NODE: u32 = 0x2;
const FDT_PROP: u32 = 0x3;
const FDT_NOP: u32 = 0x4;
const FDT_END: u32 = 0x9;
