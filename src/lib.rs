
extern crate byteorder;

use byteorder::ByteOrder;

use std::collections::HashMap;
use std::cell::Cell;

/// .DS_Store files are stored in Big Endian, so all byteorder reading of the bytes
/// Should be in BigEndian

// TODO: get rid of 0x4 Magic number. Isnt it specified by first 4 bytes of file?
// TODO: Get rid of ALL magic numbers? Or at least document each.
// TODO: are the `as usize` casts alright?

// TODO: everything should probably be usize, but lets see!
struct Block<'a>{
    data: &'a [u8],
    pos: Cell<usize> // TODO: make this Cell<usize> so a lot of functions can be &self.
}

// TODO: could datastructures than Vec/HashMap be used here?
/// Contains the data and any top-level information.
struct Allocator<'a> {
    // TODO: could this be simplified to be any readable bytes? `Read<u8>`?
    data: &'a [u8],
    offsets: Vec<u32>,
    // TODO: hard lifetime errors, possibly impossible, if we use &'a str here.
    toc: HashMap<String, u32>,
    // TODO: this could just be a Vec<u32>, as the keys are all just 2^n for n=0..32.
    // Whenever we access this vec, we could do a lookup table to see which index to use.
    // Probably a lot more code, but a lot of saved space too.
    free_list: HashMap<u32, Vec<u32>>,
}

struct Record<'a> {
    name: String,
    struct_type: &'a str,
    struct_id: &'a str,
    record_data: RecordData<'a>,
}

// TODO: implement types from:
// https://metacpan.org/pod/distribution/Mac-Finder-DSStore/DSStoreFormat.pod
enum RecordData<'a> {
    Bool(bool),
    Long(i32),
    Shor(i16),
    Blob(&'a [u8]),
}

// TODO: Better errors, BadData, NotEnoughData,
// and InvalidString could all take a &'static str, describing their errors.
enum Error {
    BadData(&'static str),
    NotEnoughData,
    InvalidString,
    // Can this be a `&'a str` somehow?
    UnkonwnStructureType(String),
    OffsetKeyDoesntExist,
}

impl<'a> Block<'a> {
    fn new(data: &'a [u8], pos: usize, size: usize) -> Result<Block, Error> {
        if data.len() < (pos + 0x4 + size) {
            Err(Error::NotEnoughData)
        } else {
            Ok(Block {
                data: &data[pos+0x4..pos+0x4+size],
                pos: Cell::new(0),
            })
        }
    }

    fn len_check(&self, size: usize) -> Result<(), Error> {
        if self.data.len() - self.pos.get() < size {
            Err(Error::NotEnoughData)
        } else {
            Ok(())
        }
    }

    fn skip(&self, amt: usize) -> Result<(), Error> {
        self.len_check(amt)?;
        self.pos.set(self.pos.get() + amt);
        Ok(())
    }

    fn read_u32(&self) -> Result<u32, Error> {
        self.len_check(4)?;
        self.pos.set(self.pos.get() + 4);
        let pos = self.pos.get();
        Ok(byteorder::BigEndian::read_u32(&self.data[pos..pos+4]))
    }

    fn read_u8(&self) -> Result<u8, Error> {
        self.len_check(1)?;
        self.pos.set(self.pos.get() + 1);
        Ok(self.data[self.pos.get()])
    }

    fn read_i16(&self) -> Result<i16, Error> {
        self.len_check(2)?;
        self.pos.set(self.pos.get() + 2);
        let pos = self.pos.get();
        Ok(byteorder::BigEndian::read_i16(&self.data[pos..pos+2]))
    }

    fn read_i32(&self) -> Result<i32, Error> {
        self.len_check(2)?;
        self.pos.set(self.pos.get() + 4);
        let pos = self.pos.get();
        Ok(byteorder::BigEndian::read_i32(&self.data[pos..pos+4]))
    }

    fn read_buf(&self, len: usize) -> Result<&'a [u8], Error> {
        self.len_check(len)?;
        self.pos.set(self.pos.get() + len);
        let pos = self.pos.get();
        Ok(&self.data[pos..pos+len])
    }

    // TODO: Small possible optimization opportinuty,
    // only has to allocate for String on big-endian machines.
    // as you can just slice::from_raw_parts the &[u8] -> &[u16] and itll just work.
    // Would need to dupe this function with #[cfg(target_endian=little/big)]
    fn read_record(&'a mut self) -> Result<Record<'a>, Error> {
        let length = self.read_u32()?;
        let filename_buf = self.read_buf(length as usize)?;
        let mut u16_buf: Vec<u16> = Vec::with_capacity(length as usize / 2);

        // FIXME: there should be a better way to do this.
        for i in 0..length/2 {
            u16_buf.push(byteorder::BigEndian::read_u16(&filename_buf[i as usize * 2..]));
        }

        let name: String = match String::from_utf16(&u16_buf) {
            Err(_) => {
                return Err(Error::InvalidString)
            },
            Ok(s) => s,
        };

        // seems like I could safely skip this?
        // TODO: Make sure this is always "Iloc".
        // TODO: could maybe do from_utf8_unchcked if we are feeling frisky.
        // We should do it, because this isnt utf8, its ASCII.
        let struct_id = match std::str::from_utf8(self.read_buf(4)?) {
            Err(_) => {
                return Err(Error::InvalidString)
            },
            Ok(s) => s,
        };

        let struct_type = match std::str::from_utf8(self.read_buf(4)?) {
            Err(_) => {
                return Err(Error::InvalidString)
            },
            Ok(s) => s,
        };

        let record_data = self.get_record_data(struct_type)?;

        Ok(Record {name, struct_type, struct_id, record_data})
    }

    fn get_record_data(&'a self, struct_type: &'a str) -> Result<RecordData<'a>, Error> {
        match struct_type {
            "bool" => Ok(RecordData::Bool(self.read_u32()? != 0)),
            "long" => Ok(RecordData::Long(self.read_i32()?)),
            "shor" => {
                self.skip(2)?;
                Ok(RecordData::Shor(self.read_i16()?))
            },
            "blob" => {
                let amt = self.read_u32()?;
                Ok(RecordData::Blob(self.read_buf(amt as usize)?))
            },
            bad => {
                Err(Error::UnkonwnStructureType(bad.to_string()))
            }
        }
    }
}



impl<'a> Allocator<'a> {
    fn new(data: &'a [u8]) -> Result<Allocator<'a>, Error> {
        let (root_offset, root_size) = read_header(data)?;
        let root = Block::new(data, root_offset as usize, root_size as usize)?;
        let offsets = read_offsets(&root)?;
        let toc = read_toc(&root)?;
        let free_list = read_free_list(&root)?;
        Ok(Allocator { data, offsets, toc, free_list})
    }

    fn get_block(&'a self, block_id: usize) -> Result<Block<'a>, Error> {
        let addr = match self.offsets.get(block_id) {
            None => {
                return Err(Error::OffsetKeyDoesntExist);
            },
            Some(addr) => addr
        };
        // TODO: document this, or reference https://0day.work blogpost
        let offset = addr & !0x1f;
        let size = 1 << (addr & 0x1f);

        // blog post has a `// +4??` at the end of this line.
        // But the block constructor does the alignment for us,
        // so I dont think it's necessary...? https://xkcd.com/979/
        Ok(Block::new(self.data, offset as usize, size as usize)?)
    }
}


/// Given a block at header position, read in the header
/// and return the (offset, size) tuple.
fn read_header<'a>(data: &'a [u8]) -> Result<(u32, u32), Error> {
    // Returning 2 u32s is kinda ugly. Better way?
    if data.len() < 32 {
        return Err(Error::NotEnoughData);
    }

    let magic1 = byteorder::BigEndian::read_u32(data);
    if magic1 != 1 {
        return Err(Error::BadData("Wrong magic bytes at start of file."));
    }

    let magic2 = byteorder::BigEndian::read_u32(&data[4..]);
    if magic2 != 0x42756431 {
        return Err(Error::BadData("Wrong magic number."));
    }

    let root_offset = byteorder::BigEndian::read_u32(&data[8..]);
    let root_size = byteorder::BigEndian::read_u32(&data[12..]);
    let root_offset2 = byteorder::BigEndian::read_u32(&data[16..]);
    if root_offset != root_offset2 {
        return Err(Error::BadData("root_offset and root_offset2 do not match."));
    }
    Ok((root_offset, root_size))
}

/// Given a block positioned at the offsets vector's start,
/// Read in the offsets.
fn read_offsets<'a>(block: &'a Block<'a>) -> Result<Vec<u32>, Error> {
    let num_offsets = block.read_u32()?;
    let mut offsets = Vec::with_capacity(num_offsets as usize);
    // unknown bytes. Seems to be always 0. Put sanity check here?
    block.skip(4)?;
    for _i in 0..num_offsets {
        offsets.push(block.read_u32()?);
    }

    // TODO: verify this math...
    // Also document this (Offsets section of https://0day.work post.)
    let bytes_to_skip = (256 - (num_offsets % 256)) * 4;
    block.skip(bytes_to_skip as usize)?;
    Ok(offsets)
}

/// Given a block positioned at the table of contents' start, read in the TOC.
fn read_toc<'a>(block: &'a Block<'a>) -> Result<HashMap<String, u32>, Error> {
    let count = block.read_u32()?;
    let mut toc = HashMap::with_capacity(count as usize);
    for _i in 0..count {
        let key_len = block.read_u8()?;
        // should be using from_utf8_unchecked here.
        let toc_key = match std::str::from_utf8(block.read_buf(key_len as usize)?) {
            Err(_) => {
                return Err(Error::InvalidString)
            },
            Ok(s) => s.to_string(),
        };
        let toc_value = block.read_u32()?;
        toc.insert(toc_key, toc_value);
    }
    Ok(toc)
}

/// Given a block positioned at the free list's start, read it in.
fn read_free_list<'a>(block: &'a Block<'a>) -> Result<HashMap<u32,Vec<u32>>, Error> {
    let mut free_list = HashMap::with_capacity(32);

    for i in 0..32 {
        let block_count = block.read_u32()?;
        let mut list = Vec::with_capacity(block_count as usize);
        for _i in 0..block_count {
            list.push(block.read_u32()?);
        }
        free_list.insert(2_u32.pow(i), list);
    }
    Ok(free_list)
}
