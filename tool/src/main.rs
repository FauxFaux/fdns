#[macro_use]
extern crate error_chain;
extern crate fdns_parse;
extern crate mio;

mod errors;
mod net;

use errors::*;

quick_main!(run);

fn run() -> Result<()> {
    net::serve_forever()
}
