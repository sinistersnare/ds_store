
extern crate byteorder;

use byteorder::ByteOrder;

use std::collections::HashMap;
use std::cell::Cell;

/// .DS_Store files are stored in Big Endian, so all byteorder reading of the bytes
/// Should be in BigEndian

// TODO: get rid of 0x4 Magic number. Isnt it specified by first 4 bytes of file?
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
    pos: usize,
    root: Box<Block<'a>>,
    offsets: Vec<u32>,
    toc: HashMap<&'a str, u32>,
    free_list: HashMap<u32, u32>,
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

enum Error<'a> {
    NotEnoughData,
    InvalidString,
    UnkonwnStructureType(&'a str),
}

impl<'a> Block<'a> {
    fn new(alloc: Allocator, pos: usize, size: usize) -> Result<Block, Error> {
        if alloc.data.len() < (pos + 0x4 + size) {
            Err(Error::NotEnoughData)
        } else {
            Ok(Block {
                data: &alloc.data[pos+0x4..pos+0x4+size],
                pos: Cell::new(0),
            })
        }
    }

    fn skip(&self, amt: usize) -> Result<(), Error> {
        if self.len_check(amt) {
            Err(Error::NotEnoughData)
        } else {
            self.pos.set(self.pos.get() + amt);
            Ok(())
        }
    }

    fn len_check(&self, size: usize) -> bool {
        self.data.len() - self.pos.get() < size
    }

    fn read_u32(&self) -> Result<u32, Error> {
        if self.len_check(4) {
            Err(Error::NotEnoughData)
        } else {
            self.pos.set(self.pos.get() + 4);
            let pos = self.pos.get();
            Ok(byteorder::BigEndian::read_u32(&self.data[pos..pos+4]))
        }
    }

    fn read_u8(&self) -> Result<u8, Error> {
        if self.len_check(1) {
            Err(Error::NotEnoughData)
        } else {
            self.pos.set(self.pos.get() + 1);
            Ok(self.data[self.pos.get()])
        }
    }

    fn read_i16(&self) -> Result<i16, Error> {
        if self.len_check(2) {
            Err(Error::NotEnoughData)
        } else {
            self.pos.set(self.pos.get() + 2);
            let pos = self.pos.get();
            Ok(byteorder::BigEndian::read_i16(&self.data[pos..pos+2]))
        }
    }

    fn read_i32(&self) -> Result<i32, Error> {
        if self.len_check(2) {
            Err(Error::NotEnoughData)
        } else {
            self.pos.set(self.pos.get() + 4);
            let pos = self.pos.get();
            Ok(byteorder::BigEndian::read_i32(&self.data[pos..pos+4]))
        }
    }

    fn read_buf(&self, len: usize) -> Result<&'a [u8], Error> {
        if self.len_check(len) {
            Err(Error::NotEnoughData)
        } else {
            self.pos.set(self.pos.get() + len);
            let pos = self.pos.get();
            Ok(&self.data[pos..pos+len])
        }
    }

    // Small optimization opportinuty , only has to allocate for String on big-endian machines.
    // as you can just slice::from_raw_parts the &[u8] -> &[u16] and itll just work.
    // consider that a TODO
    fn read_record(&mut self) -> Result<Record<'a>, Error> {
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

        let record_data = match struct_type {
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
                Err(Error::UnkonwnStructureType(bad))
            }
        }?;

        Ok(Record {name, struct_type, struct_id, record_data})
    }
}
