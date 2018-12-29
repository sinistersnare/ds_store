# A Rusty `.DS_Store` Parser #

## Get The Library! ##

add something like this to your `Cargo.toml` file:

```toml
[dependencies]
ds_store = "0.1"
```

## Usage ##

```rust
extern crate ds_store;

use std::{io::Read, fs::File};
use ds_store::{DsStore, Record};

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
    let records: &Vec<Record> = store.records();
    records.iter().for_each(|r| println!("{:?}", r));
    println!("printed {:?} records", records.len());
}
```

This example is replicated in `examples/basic.rs`. Call it with `$ cargo run --example basic examples/basic.DS_Store`

## Rust Version ##

Should be 2015 edition compatible!

## License ##

This code is distributed under the ***MIT license***.
Please see the LICENSE.md file for information.


## TODO ##

Looking at this list, I probably should not have released the library so early! I was so excited though!
Oh well, I guess I better get to improving it!

* Describe what DS_Store files are in the README!
* Address the various TODOs within the code.
* Document everything!
* Rigorous testing? Probably!
    * Make sure to test with background images and all sorts of stuff.
* Better API? What do _you_ want to do with DS_Store files? Let me know! Make an issue!
* Make no_std compatible? Probably not gonna happen. std types too nice.
* Fill out Cargo.toml metadata with docs location and stuff.
* Should I exclude the examples from the cargo manifest? So that there isnt so many useless kilobytes being downloaded with the crate. People who want the examples probably just clone the library right?
* Creation/manipulation of `.DS_Store` files?????
