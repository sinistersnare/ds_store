extern crate ds_store;

use std::collections::HashMap;
use std::{io::Read, fs::File};
use ds_store::{DsStore, RecordValue};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Incorrect usage! `./binary_path /path/to/.DS_Store");
        return;
    }

    let mut file = File::open(&args[1]).expect("Could not open file.");
    let mut buf: Vec<u8> = vec![];
    file.read_to_end(&mut buf).expect("Could not read file to end.");

    let store: DsStore = match DsStore::new(&buf) {
        Ok(s) => s,
        Err(e) => {
            println!("Could not construct the DS_Store: {:?}", e);
            return;
        }
    };
    let records: &HashMap<String, HashMap<&str, RecordValue>> = store.contents();
    records.iter().for_each(|r| println!("{:?}", r));
    println!("printed {:?} records", records.len());
}
