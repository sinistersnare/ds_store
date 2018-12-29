
extern crate byteorder;

use crate::allocator::{Allocator};
pub use crate::allocator::{Directory, Record};
pub mod allocator;


// TODO: Better errors NotEnoughData, and InvalidString could all take a &'static str, describing their errors.
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

pub struct DsStore<'a> {
    directory: Directory<'a>,
}

impl<'a> DsStore<'a> {
    pub fn new(file_data: &'a [u8]) -> Result<DsStore<'a>, Error> {
        let allocator = Allocator::new(file_data)?;
        let directory = allocator.traverse()?;
        Ok(DsStore {directory})
    }

    pub fn records(&self) -> &Vec<Record<'a>> {
        &self.directory.records
    }
}
