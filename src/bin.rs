use std::io::Read;


use ds_store::allocator::Allocator;


fn main() {
	let args: Vec<String> = std::env::args().collect();
	if args.len() != 2 {
		println!("Incorrect usage! `./binary_path /path/to/.DS_Store");
		return;
	}
	let mut file = std::fs::File::open(&args[1]).unwrap();
	let mut buf: Vec<u8> = vec![];
	file.read_to_end(&mut buf);
	let a = match Allocator::new(&buf) {
		Ok(a) => a,
		Err(e) => {
			println!("Got error `{:?}`, oh no!", e);
			return;
		}
	};
	let records = match a.traverse() {
		Ok(r) => r,
		Err(e) => {
			println!("God error `{:?}`, oh no!", e);
			return;
		}
	};
	for record in &records {
		println!("Record: {:?}", record);
	}
	println!("printed {:?} records", records.len());
}
