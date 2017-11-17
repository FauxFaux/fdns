extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate mio;

#[macro_use]
extern crate nom;

mod errors;
mod net;
mod parse;

use errors::*;

quick_main!(run);

fn run() -> Result<()> {
    net::serve_forever()
}

fn usize_from(val: u16) -> usize {
    val as usize
}
