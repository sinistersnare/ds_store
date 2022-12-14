
use std::collections::HashMap;
use byteorder::{ByteOrder, BigEndian};
use crate::Error;

// TODO verify `as usize` casts, or place them where they truly belong (where they are created, not when used).

#[derive(Debug)]
pub struct Directory<'a> {
    num_internals: u32,
    num_nodes: u32,
    num_records: u32,

    /// A map from file/directory to its information
    pub contents: HashMap<String, HashMap<&'a str, RecordValue<'a>>>,
}

// TODO: Better strongly type these. Instead of having so many slices, parse more. Also, PList variant.
#[derive(Debug)]
pub enum RecordValue<'a> {
    Background(BackgroundType),
    Style(StyleType),
    Bool(bool),
    Slice(&'a [u8]),
    String(String),
    I16(i16),
    I32(i32),
    I64(i64),
    U32(u32),
    DateTime(chrono::DateTime<chrono::Utc>),
}

#[derive(Debug)]
pub enum BackgroundType {
    // Parsed as: FourCharCode "DefB", followed by eight unknown bytes, probably garbage.
    Default,
    // Parsed as: FourCharCode "ClrB", followed by an RGB value in six bytes, followed by two unknown bytes.
    SolidColor(u16, u16, u16),
    // Parsed as: FourCharCode "PctB", followed by the the length of the blob stored in the 'pict' record,
    // followed by four unknown bytes. The 'pict' record points to the actual background image.
    Picture(u32),
}

/// How a directory is viewed in the finder.
/// Icon view, Column/Browser view, List view, and Cover Flow view.
#[derive(Debug)]
pub enum StyleType {
    /// represented as "icnv" in the .DS_Store file.
    Icon,
    /// represented as "clmv" in the .DS_Store file.
    ColumnBrowser,
    /// represented as "Nlsv" in the .DS_Store file.
    List,
    /// represented as "Flwv" in the .DS_Store file.
    CoverFlow,
}

/// A Block is a u8-slice, with methods for reading from it in _Big-Endian format_.
struct Block<'a>(&'a [u8]);

impl<'a> Block<'a> {
    fn new(data: &'a [u8], offset: usize, size: usize) -> Result<Block<'a>, Error<'a>> {
        if data.len() < offset+0x4+size {
            Err(Error::NotEnoughData)
        } else {
            Ok(Block(&data[offset+0x4..offset+0x4+size]))
        }
    }

    fn len_check(&self, amt: usize) -> Result<(), Error<'a>> {
        if self.0.len() < amt {
            Err(Error::NotEnoughData)
        } else {
            Ok(())
        }
    }

    fn skip(&mut self, amt: usize) -> Result<(), Error<'a>> {
        self.len_check(amt)?;
        self.0 = &self.0[amt..];
        Ok(())
    }

    fn read_bool(&mut self) -> Result<bool, Error<'a>> {
        self.len_check(1)?;
        let ret = self.0[0] == 1;
        self.0 = &self.0[1..];
        Ok(ret)
    }

    fn read_i16(&mut self) -> Result<i16, Error<'a>> {
        self.len_check(2)?;
        let ret = Ok(BigEndian::read_i16(self.0));
        self.0 = &self.0[2..];
        ret
    }

    fn read_u16(&mut self) -> Result<u16, Error<'a>> {
        self.len_check(2)?;
        let ret = Ok(BigEndian::read_u16(self.0));
        self.0 = &self.0[2..];
        ret
    }

    fn read_i32(&mut self) -> Result<i32, Error<'a>> {
        self.len_check(4)?;
        let ret = Ok(BigEndian::read_i32(self.0));
        self.0 = &self.0[4..];
        ret
    }

    fn read_u32(&mut self) -> Result<u32, Error<'a>> {
        self.len_check(4)?;
        let ret = Ok(BigEndian::read_u32(self.0));
        self.0 = &self.0[4..];
        ret
    }

    fn read_i64(&mut self) -> Result<i64, Error<'a>> {
        self.len_check(8)?;
        let ret = Ok(BigEndian::read_i64(self.0));
        self.0 = &self.0[4..];
        ret
    }

    fn read_exact(&mut self, data: &'static [u8], err_msg: &'static str) -> Result<(), Error<'a>> {
        self.len_check(data.len())?;
        let unconfirmed = &self.0[..data.len()];
        self.0 = &self.0[data.len()..];
        if unconfirmed != data {
            Err(Error::BadData(err_msg))
        } else {
            Ok(())
        }
    }

    fn read_buf(&mut self, amt: usize) -> Result<&'a [u8], Error<'a>> {
        self.len_check(amt)?;
        let (left, right) = self.0.split_at(amt);
        self.0 = right;
        Ok(left)
    }

    /// Reads a 4-byte `length` and then reads (`length*2`)-bytes to create a `String`.
    fn read_utf16(&mut self) -> Result<String, Error<'a>> {
        // TODO: Small possible optimization opportinuty,
        // only has to allocate for String on big-endian machines.
        // as you can just slice::from_raw_parts the &[u8] -> &[u16] and itll just work.
        // Would need to dupe this function with #[cfg(target_endian=little/big)]
        let file_name_length = self.read_u32()?;
        let mut u16_buf: Vec<u16> = Vec::with_capacity(file_name_length as usize * 2);

        for _ in 0..file_name_length {
            u16_buf.push(self.read_u16()?);
        }

        match String::from_utf16(&u16_buf) {
            Err(_) => Err(Error::InvalidString),
            Ok(s) => Ok(s),
        }
    }

    // Reads a 4-byte length, then length-bytes of self.
    fn read_blob(&mut self) -> Result<&'a [u8], Error<'a>> {
        let length = self.read_u32()?;
        Ok(self.read_buf(length as usize)?)
    }

    fn read_record(&mut self, records: &mut HashMap<String, HashMap<&'a str, RecordValue<'a>>>) -> Result<(), Error<'a>> {
        let file_name = self.read_utf16()?;
        let mut metadata = records.entry(file_name).or_insert(HashMap::new());
        self.read_record_info(&mut metadata)?;
        Ok(())
    }

    fn read_date_time(&mut self) -> Result<chrono::DateTime<chrono::Utc>, Error<'a>> {
        // number that when added to a Mac-epoch integer, converts it to a Unix-epoch integer.
        // TODO: MAKE SURE THIS WORKS?
        const CONVERTER: i64 = 2082844800;
        let raw = self.read_i64()?;
        println!("DateTime raw: {}", raw);
        Ok(chrono::DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(raw + CONVERTER, 0), chrono::Utc))
    }

    // TODO: better strongly type the RecordValues. "bwsp" is actually a plist. Many blobs are meaningful.
    fn read_record_info(&mut self, records: &mut HashMap<&'a str, RecordValue<'a>>) -> Result<(), Error<'a>> {
        let structure_type: &'a [u8] = self.read_buf(4)?;
        let record_value: RecordValue = match structure_type {
            b"BKGD" => {
                self.read_exact(b"blob", "\"BKGD\" only takes blobs describing the background.")?;
                match self.read_buf(4)? {
                    b"DefB" => {
                        self.skip(8)?;
                        Ok(RecordValue::Background(BackgroundType::Default))
                    },
                    b"ClrB" => {
                        let r = self.read_u16()?;
                        let g = self.read_u16()?;
                        let b = self.read_u16()?;
                        self.skip(2)?; // unknown bytes. Seemingly not alpha?
                        Ok(RecordValue::Background(BackgroundType::SolidColor(r,g,b)))
                    },
                    b"PctB" => {
                        let picture_property_blob_length = self.read_u32()?;
                        self.skip(4)?;
                        Ok(RecordValue::U32(picture_property_blob_length))
                    },
                    other => Err(Error::UnkonwnStructureType(other))
                }
            },
            // TODO: read_blob function.
            b"ICVO" => {
                self.read_exact(b"bool", "\"ICVO\" only takes bool")?;
                Ok(RecordValue::Bool(self.read_bool()?))
            },
            b"Iloc" => {
                self.read_exact(b"blob", "\"Iloc\" only takes bool")?;
                self.read_exact(&[0,0,0,16], "\"Iloc\" only takes a 16-byte blob.")?;
                Ok(RecordValue::Slice(self.read_buf(16)?))
            },
            b"LSVO" => {
                self.read_exact(b"bool", "\"LSVO\" only takes bool")?;
                Ok(RecordValue::Bool(self.read_bool()?))
            },
            b"bwsp" => {
                self.read_exact(b"blob", "\"bwsp\" only takes blob")?;
                Ok(RecordValue::Slice(self.read_blob()?))
            },
            b"cmmt" => {
                self.read_exact(b"ustr", "\"cmmt\" only takes ustr")?;
                Ok(RecordValue::String(self.read_utf16()?))
            },
            b"dilc" => {
                self.read_exact(b"blob", "\"dilc\" only takes blob")?;
                self.read_exact(&[0,0,0,32], "\"dilc\" only takes a 32-byte blob.")?;
                Ok(RecordValue::Slice(self.read_buf(32)?))
            },
            b"dscl" => {
                self.read_exact(b"bool", "\"dscl\" only takes bool")?;
                Ok(RecordValue::Bool(self.read_bool()?))
            },
            b"extn" => {
                self.read_exact(b"ustr", "\"extn\" only takes ustr")?;
                Ok(RecordValue::String(self.read_utf16()?))
            },
            b"fwi0" => {
                self.read_exact(b"blob", "\"fwi0\" only takes blob")?;
                self.read_exact(&[0,0,0,16], "\"fwi0\" only takes 16-byte blob")?;
                Ok(RecordValue::Slice(self.read_buf(16)?))
            },
            b"fwsw" => {
                self.read_exact(b"long", "\"fwsw\" only takes long")?;
                Ok(RecordValue::I32(self.read_i32()?))
            },
            b"fwvh" => {
                self.read_exact(b"shor", "\"fwvh\" only takes shor")?;
                self.skip(2)?; // shor is 4 bytes long, but only 16 bit. skip 2 bytes.
                Ok(RecordValue::I16(self.read_i16()?))
            },
            b"GRP0" => {
                self.read_exact(b"ustr", "\"GRP0\" only takes ustr")?;
                Ok(RecordValue::String(self.read_utf16()?))
            },
            b"icgo" => {
                self.read_exact(b"blob", "\"icgo\" only takes blob")?;
                self.read_exact(&[0,0,0,8], "\"icgo\" only takes 8-byte blob")?;
                Ok(RecordValue::Slice(self.read_buf(8)?))
            },
            b"icsp" => {
                self.read_exact(b"blob", "\"icsp\" only takes blob")?;
                self.read_exact(&[0,0,0,8], "\"icsp\" only takes 8-byte blob")?;
                Ok(RecordValue::Slice(self.read_buf(8)?))
            },
            b"icvo" => {
                self.read_exact(b"blob", "\"icvo\" only takes blob")?;
                let blob = self.read_blob()?;

                if blob.len() == 18 || blob.len() == 26 {
                    Ok(RecordValue::Slice(blob))
                } else {
                    Err(Error::BadData("\"icvo\" only takes 18 or 26 byte blob."))
                }
            },
            b"icvp" => {
                self.read_exact(b"blob", "\"icvp\" only takes blob")?;
                Ok(RecordValue::Slice(self.read_blob()?))
            },
            b"icvt" => {
                self.read_exact(b"shor", "\"icvt\" only takes shor")?;
                self.skip(2)?;
                Ok(RecordValue::I16(self.read_i16()?))
            },
            b"info" => {
                self.read_exact(b"blob", "\"info\" only takes blob")?;
                let blob = self.read_blob()?;
                if blob.len() == 40 || blob.len() == 48 {
                    Ok(RecordValue::Slice(blob))
                } else {
                    Err(Error::BadData("\"info\" only takes 40 or 48 byte blob."))
                }
            },
            b"logS" | b"lg1S" => {
                self.read_exact(b"comp", "\"logS\"/\"lg1S\" only takes comp")?;
                Ok(RecordValue::I64(self.read_i64()?))
            },
            b"lssp" => {
                self.read_exact(b"blob", "\"lssp\" only takes blob")?;
                self.read_exact(&[0,0,0,8], "\"lssp\" only takes 8-byte blob")?;
                Ok(RecordValue::Slice(self.read_buf(8)?))
            },
            b"lsvo" => {
                self.read_exact(b"blob", "\"lsvo\" only takes blob")?;
                self.read_exact(&[0,0,0,76], "\"lsvo\" only takes 76-byte blob")?;
                Ok(RecordValue::Slice(self.read_buf(76)?))
            },
            b"lsvt" => {
                self.read_exact(b"shor", "\"lsvt\" only take shor")?;
                self.skip(2)?;
                Ok(RecordValue::I16(self.read_i16()?))
            },
            b"lsvp" => {
                self.read_exact(b"blob", "\"lsvp\" only takes blob")?;
                Ok(RecordValue::Slice(self.read_blob()?))
            },
            b"lsvP" => {
                self.read_exact(b"blob", "\"lsvP\" only takes blob")?;
                Ok(RecordValue::Slice(self.read_blob()?))
            },
            b"modD" | b"moDD" => {
                self.read_exact(b"dutc", "\"modD\"/\"moDD\" only takes dutc")?;
                Ok(RecordValue::DateTime(self.read_date_time()?))
            },
            b"phyS" | b"ph1S" => {
                self.read_exact(b"comp", "\"phyS\"/\"ph1S\" only takes comp")?;
                Ok(RecordValue::I64(self.read_i64()?))
            },
            b"pict" => {
                // I really hope that `pic` is a regular blob,
                // but the docs are unclear if we need to get the length from the 'BKGD' key.
                self.read_exact(b"blob", "\"pict\" only takes blob")?;
                //  TODO: maybe do a verify, get the BKGD key and check they are the same?
                Ok(RecordValue::Slice(self.read_blob()?))
            },
            b"vSrn" => {
                self.read_exact(b"long", "\"vSrn\" only takes long")?;
                Ok(RecordValue::I32(self.read_i32()?))
            },
            b"vstl" => {
                self.read_exact(b"type", "\"vstl\" only takes type")?;
                // let view_type = self.read_buf(4)?;
                match self.read_buf(4)? {
                    b"icnv" => Ok(RecordValue::Style(StyleType::Icon)),
                    b"clmv" => Ok(RecordValue::Style(StyleType::ColumnBrowser)),
                    b"Nlsv" => Ok(RecordValue::Style(StyleType::List)),
                    b"Flwv" => Ok(RecordValue::Style(StyleType::CoverFlow)),
                    other => Err(Error::UnkonwnStructureType(other)),
                }
            },
            b"ptbL" => {
                self.read_exact(b"ustr", "\"ptbL\" only takes ustr")?;
                Ok(RecordValue::String(self.read_utf16()?))
            },
            b"ptbN" => {
                self.read_exact(b"ustr", "\"ptbN\" only takes ustr")?;
                Ok(RecordValue::String(self.read_utf16()?))
            },
            other => Err(Error::UnkonwnStructureType(other)),
        }?;
        // should be impossible to hit error case,
        // but is getting annoyed by people saying I shouldnt use unsafe code worth it? :p joking of course.
        let type_str = match std::str::from_utf8(structure_type) {
            Ok(s) => s,
            Err(_) => { return Err(Error::InvalidString); }
        };
        // TODO: maybe check if dupe, and Err if so?
        records.insert(type_str, record_value);
        Ok(())
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
    pub fn new(data: &'a [u8]) -> Result<Allocator<'a>, Error<'a>> {
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

    fn get_block(&self, block_id: u32) -> Result<Block<'a>, Error<'a>> {
        if self.offsets.len() < block_id as usize {
            return Err(Error::BlockDoesntExist);
        }
        let address = self.offsets[block_id as usize];
        // Go code does some type casting to i32 here, should I?
        let offset = address & !0x1f;
        let size = 1 << (address & 0x1f);
        Block::new(self.data, offset as usize, size)
    }

    fn read_prelude(info_block: &mut Block<'a>) -> Result<(u32, u32), Error<'a>> {
        info_block.read_exact(b"Bud1", "Magic number is wrong.")?;

        let offset = info_block.read_u32()?;
        let size = info_block.read_u32()?;
        let offset_check = info_block.read_u32()?;

        if offset != offset_check {
            return Err(Error::BadData("Offset check failed"));
        }
        Ok((offset, size))
    }

    fn read_offsets(info_block: &mut Block<'a>) -> Result<Vec<u32>, Error<'a>> {
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

    fn read_dsdb_location(info_block: &mut Block<'a>) -> Result<u32, Error<'a>> {
        // Amount of entries in the TOC.
        info_block.read_exact(&[0,0,0,1], "I Thought there should only be 1 TOC entry...")?;
        info_block.read_exact(&[4], "Looks like \"DSDB\" is not the only key...")?;
        info_block.read_exact(b"DSDB", "I thought only key was \"DSDB\"...")?;
        Ok(info_block.read_u32()?) // value!
    }

    fn read_free_list(info_block: &mut Block<'a>) -> Result<Vec<Vec<u32>>, Error<'a>>  {
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

    pub fn traverse(&self) -> Result<Directory<'a>, Error<'a>> {
        let mut root_block = self.get_block(self.dsdb_location)?;
        let root_node = root_block.read_u32()?;
        let num_internals = root_block.read_u32()?;
        let num_records = root_block.read_u32()?;
        let num_nodes = root_block.read_u32()?;

        root_block.read_exact(&[0,0, 0x10, 0], "Expected 0x1000, found not that.")?;
        let mut contents = HashMap::new();
        self.traverse_tree(root_node, &mut contents)?;
        Ok(Directory {num_internals, num_records, num_nodes, contents})
    }

    fn traverse_tree(&self, block_id: u32, contents: &mut HashMap<String, HashMap<&'a str, RecordValue<'a>>>) -> Result<(), Error<'a>> {
        let mut current_block = self.get_block(block_id)?;

        let pair_count = current_block.read_u32()?;
        if pair_count == 0 {
            // We are at a leaf! Congratulations!
            let count = current_block.read_u32()?;
            for _ in 0..count {
                current_block.read_record(contents)?;
            }
        } else {
            // Internal node of the B-Tree!
            for _ in 0..pair_count {
                let child = current_block.read_u32()?;
                self.traverse_tree(child, contents)?;
                current_block.read_record(contents)?;
            }
        }
        Ok(())
    }
}
