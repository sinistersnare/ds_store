
extern crate byteorder;
extern crate chrono;

use std::collections::HashMap;
use crate::allocator::{Allocator};
pub use crate::allocator::{Directory, RecordValue};
pub mod allocator;


// TODO: Better errors NotEnoughData, and InvalidString could all take a &'static str, describing their errors.
// TODO: `impl std::error::Error`?
#[derive(Debug)]
pub enum Error<'a> {
    BadData(&'static str),
    NotEnoughData,
    BlockDoesntExist,
    InvalidString,
    UnkonwnStructureType(&'a [u8]),
    UnsupportedStructureType(&'a [u8])
}

pub struct DsStore<'a> {
    directory: Directory<'a>,
}

impl<'a> DsStore<'a> {
    pub fn new(file_data: &'a [u8]) -> Result<DsStore<'a>, Error<'a>> {
        let allocator = Allocator::new(file_data)?;
        let contents: Directory<'a> = allocator.traverse()?;
        Ok(DsStore {directory: contents})
    }

    pub fn contents(&self) -> &HashMap<String, HashMap<&'a str, RecordValue<'a>>> {
        &self.directory.contents
    }
}
