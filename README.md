# A Rusty `.DS_Store` Parser #


[![Chrono on crates.io][cratesio-image]][cratesio]
[![Chrono on docs.rs][docsrs-image]][docsrs]
[cratesio-image]: https://img.shields.io/crates/v/ds_store.svg
[cratesio]: https://crates.io/crates/ds_store
[docsrs-image]: https://docs.rs/ds_store/badge.svg
[docsrs]: https://docs.rs/ds_store

## Get The Library! ##

add something like this to your `Cargo.toml` file:

```toml
[dependencies]
ds_store = "0.1"
```

## Usage ##

```rust
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
```

This example is replicated in `examples/basic.rs`. Call it with `$ cargo run --example basic examples/basic.DS_Store`

## Rust Version ##

Should be 2015 edition compatible!

## License ##

This code is distributed under the ***MIT license***.
Please see the LICENSE.md file for information.


## What is a `.DS_Store` file? ##

The _Desktop Services Store_ is mostly just deleted without a second thought. However, such files can be helpful! What are they really for?

Put simply, `.DS_Store` files are used on MacOS Computers to describe the contents of the directory they are in.
`.DS_Store`'s are created, maintained, and read by the Finder application to properly render directories.
The things that this file describes includes properties set in the directory options, file icons, directory background color or image, and many more things.

The file has 3 important sections. First, the prelude, which gives information about where to find the main information block of the file. Second, the information block, containing bookkeeping information for the data-section. Finally, the data section, holding the actual metadata of the directory.


## TODO ##

Looking at this list, I probably should not have released the library so early! I was so excited though!
Oh well, I guess I better get to improving it!

* Address the various TODOs within the code.
* Document everything!
* Rigorous testing? Probably!
    * Make sure to test with background images and all sorts of stuff.
* Better API? What do _you_ want to do with DS_Store files? Let me know! Make an issue!
* Creation/manipulation of `.DS_Store` files?????
* Make no_std compatible? Probably not gonna happen. std types too nice.
* Add logging, to log assumptions made being proved wrong (like 'icgo' record not being `0x0000000000000004`)
