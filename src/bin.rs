use std::io::Read;
use std::fs::File;

use ds_store::DsStore;

fn main() {
	let args: Vec<String> = std::env::args().collect();
	if args.len() != 2 {
		println!("Incorrect usage! `./binary_path /path/to/.DS_Store");
		return;
	}

	let mut file = File::open(&args[1]).expect("Could not open file.");
	let mut buf: Vec<u8> = vec![];
	file.read_to_end(&mut buf).expect("Could not read file to end.");

	let store = DsStore::new(&buf).expect("Could not create DS_Store object.");
	for rec in store.records() {
		println!("Record {{data: {:?},\tfile_name: {:?}}}", rec.data, rec.file_name);
	}
	println!("printed {:?} records", store.records().len());
}
