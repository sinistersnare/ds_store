

use byteorder::{ByteOrder, BigEndian};

// TODO verify `as usize` casts, or place them where they truly belong (where they are created, not when used).

// TODO: Better errors, BadData, NotEnoughData,
// and InvalidString could all take a &'static str, describing their errors.
#[derive(Debug)]
pub enum Error {
    BadData(&'static str),
    NotEnoughData,
    BlockDoesntExist,
    InvalidString,
    // Can this be a `&'a str` somehow?
    UnkonwnStructureType(String),
    // OffsetKeyDoesntExist,
}

#[derive(Debug)]
pub struct Directory<'a> {
    num_internals: u32,
    num_nodes: u32,
    pub num_records: u32,
    pub records: Vec<Record<'a>>,
}

#[derive(Debug)]
pub struct Record<'a> {
    file_name: String,
    data: RecordData<'a>,
}

#[derive(Debug)]
enum RecordData<'a> {
    Bool(bool),
    Long(i32),
    Shor(i16),
    Blob(&'a [u8]),
}

struct Block<'a>(&'a [u8]);

impl<'a> Block<'a> {
    fn new(data: &'a [u8], offset: usize, size: usize) -> Result<Block<'a>, Error> {
        if data.len() < offset+0x4+size {
            Err(Error::NotEnoughData)
        } else {
            Ok(Block(&data[offset+0x4..offset+0x4+size]))
        }
    }

    fn len_check(&self, amt: usize) -> Result<(), Error> {
        if self.0.len() < amt {
            Err(Error::NotEnoughData)
        } else {
            Ok(())
        }
    }

    fn skip(&mut self, amt: usize) -> Result<(), Error> {
        self.len_check(amt)?;
        self.0 = &self.0[amt..];
        Ok(())
    }

    fn read_i16(&mut self) -> Result<i16, Error> {
        self.len_check(2)?;
        let ret = Ok(BigEndian::read_i16(self.0));
        self.0 = &self.0[2..];
        ret
    }

    fn read_i32(&mut self) -> Result<i32, Error> {
        self.len_check(4)?;
        let ret = Ok(BigEndian::read_i32(self.0));
        self.0 = &self.0[4..];
        ret
    }

    fn read_u32(&mut self) -> Result<u32, Error> {
        self.len_check(4)?;

        let ret = Ok(BigEndian::read_u32(self.0));
        self.0 = &self.0[4..];
        ret
    }

    fn read_exact(&mut self, data: &'static [u8], err_msg: &'static str) -> Result<(), Error> {
        self.len_check(data.len())?;
        let unconfirmed = &self.0[..data.len()];
        self.0 = &self.0[data.len()..];
        if unconfirmed != data {
            Err(Error::BadData(err_msg))
        } else {
            Ok(())
        }
    }

    fn read_buf(&mut self, amt: usize) -> Result<&'a [u8], Error> {
        self.len_check(amt)?;
        let (left, right) = self.0.split_at(amt);
        self.0 = right;
        Ok(left)
        // let ret = &self.0[..amt];
        // self.0 = &self.0[amt..];
        // Ok(ret) // TODO
    }


    // TODO: Small possible optimization opportinuty,
    // only has to allocate for String on big-endian machines.
    // as you can just slice::from_raw_parts the &[u8] -> &[u16] and itll just work.
    // Would need to dupe this function with #[cfg(target_endian=little/big)]
    fn read_record(&mut self) -> Result<Record<'a>, Error> {
        let length = self.read_u32()?;
        let filename_buf = self.read_buf(length as usize)?;
        let file_name = {
            let mut u16_buf: Vec<u16> = Vec::with_capacity(length as usize / 2);

            // FIXME: there should be a better way to do this.
            // Other than the BE/LE optimization mentioned above.
            for i in 0..length/2 {
                u16_buf.push(byteorder::BigEndian::read_u16(&filename_buf[i as usize * 2..]));
            }

            match String::from_utf16(&u16_buf) {
                Err(_) => {
                    return Err(Error::InvalidString);
                },
                Ok(s) => s,
            }
        };

        // seems like I could safely skip this?
        // TODO: Make sure this is always "Iloc".
        // TODO: could maybe do from_utf8_unchcked if we are feeling frisky.
        // We should do it, because this isnt utf8, its ASCII.
        self.read_exact(b"Iloc", "Struct ID was not \"Iloc\".")?;

        let struct_type = match std::str::from_utf8(self.read_buf(4)?) {
            Err(_) => {
                return Err(Error::InvalidString)
            },
            Ok(s) => s,
        };

        let data = self.get_record_data(struct_type)?;

        Ok(Record {file_name, data})
    }

    fn get_record_data(&mut self, struct_type: &'a str) -> Result<RecordData<'a>, Error> {
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

pub struct Allocator<'a> {
    /// The whole data to be partitioned into blocks by the allocator.
    data: &'a [u8],

    /// The offsets to each block(?) (TODO write this.)
    pub offsets: Vec<u32>,
    /// It is a 'table of contents', but it seems that there is only ever 1 entry, "DSDB".
    pub dsdb_location: u32,
    /// locations of data allocted by the buddy-allocator. (TODO: write this.)
    pub free_list: Vec<Vec<u32>>,
}

impl<'a> Allocator<'a> {
    /// Create a new alloctor, initalizing all important data needed for traversal.
    pub fn new(data: &'a [u8]) -> Result<Allocator<'a>, Error> {
        if &data[0..4] != &[0,0,0,1] {
            // creating a block offsets by 4 bytes, so check the first 4 here.
            return Err(Error::BadData("First 4 bytes must be `1`."));
        }
        let mut prelude_block = Block::new(data, 0, 32)?;

        let (info_block_offset, info_block_size) = Allocator::read_prelude(&mut prelude_block)?;
        let mut info_block = Block::new(data, info_block_offset as usize, info_block_size as usize)?;

        let offsets = Allocator::read_offsets(&mut info_block)?;
        let dsdb_location = Allocator::read_dsdb_location(&mut info_block)?;
        let free_list = Allocator::read_free_list(&mut info_block)?;

        Ok(Allocator {data, offsets, dsdb_location, free_list}) // allocator should be fully allocated here.
    }

    fn get_block(&self, block_id: u32) -> Result<Block<'a>, Error> {
        if self.offsets.len() < block_id as usize {
            return Err(Error::BlockDoesntExist);
        }
        let address = self.offsets[block_id as usize];
        // Go code does some type casting to i32 here, should I?
        let offset = address & !0x1f;
        let size = 1 << (address & 0x1f);
        Block::new(self.data, offset as usize, size)
    }

    fn read_prelude(info_block: &mut Block<'a>) -> Result<(u32, u32), Error> {
        info_block.read_exact(b"Bud1", "Magic number is wrong.")?;

        let offset = info_block.read_u32()?;
        let size = info_block.read_u32()?;
        let offset_check = info_block.read_u32()?;

        if offset != offset_check {
            return Err(Error::BadData("Offset check failed"));
        }
        Ok((offset, size))
    }

    fn read_offsets(info_block: &mut Block<'a>) -> Result<Vec<u32>, Error> {
        let num_offsets = info_block.read_u32()?;
        let mut offsets = Vec::with_capacity(num_offsets as usize);
        // Documented as unknown bytes, always observed as 0.
        info_block.read_exact(&[0,0,0,0], "Thought these should always be 0???")?;
        for _i in 0..num_offsets {
            offsets.push(info_block.read_u32()?);
        }

        // TODO: verify this math...
        // Also document this (Offsets section of https://0day.work post.)
        let bytes_to_skip = (256 - (num_offsets % 256)) * 4;
        info_block.skip(bytes_to_skip as usize)?;
        Ok(offsets)
    }

    fn read_dsdb_location(info_block: &mut Block<'a>) -> Result<u32, Error> {
        // Amount of entries in the TOC.
        info_block.read_exact(&[0,0,0,1], "I Thought there should only be 1 TOC entry...")?;
        info_block.read_exact(&[4], "Looks like \"DSDB\" is not the only key...")?;
        info_block.read_exact(b"DSDB", "I thought only key was \"DSDB\"...")?;
        Ok(info_block.read_u32()?) // value!
    }

    fn read_free_list(info_block: &mut Block<'a>) -> Result<Vec<Vec<u32>>, Error>  {
        let mut free_list = Vec::with_capacity(32);
        for _ in 0..=31 {
            let block_count = info_block.read_u32()?;
            let mut list = Vec::with_capacity(block_count as usize);
            for _ in 0..block_count {
                list.push(info_block.read_u32()?);
            }
            free_list.push(list);
        }
        Ok(free_list)
    }

    pub fn traverse(&self) -> Result<Directory<'a>, Error> {
        let mut root_block = self.get_block(self.dsdb_location)?;
        let root_node = root_block.read_u32()?;
        let num_internals = root_block.read_u32()?;
        let num_records = root_block.read_u32()?;
        let num_nodes = root_block.read_u32()?;

        root_block.read_exact(&[0,0, 0x10, 0], "Expected 0x1000, found not that.")?;
        let mut records = Vec::with_capacity(num_records as usize);
        self.traverse_tree(root_node, &mut records)?;
        Ok(Directory {num_internals, num_records, num_nodes, records})
    }

    fn traverse_tree(&self, block_id: u32, records: &mut Vec<Record<'a>>) -> Result<(), Error> {
        let mut current_block = self.get_block(block_id)?;

        let pair_count = current_block.read_u32()?;
        if pair_count == 0 {
            // We are at a leaf! Congratulations!
            let count = current_block.read_u32()?;
            for _ in 0..count {
                records.push(current_block.read_record()?);
            }
        } else {
            // Internal node of the B-Tree!
            for _ in 0..pair_count {
                let child = current_block.read_u32()?;
                self.traverse_tree(child, records)?;
                let current_record = current_block.read_record()?;
                records.push(current_record);
            }
        }
        Ok(())
    }
}
