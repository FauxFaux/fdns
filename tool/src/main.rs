extern crate failure;
extern crate fdns_format;
extern crate mio;

mod net;

use failure::Error;

fn main() -> Result<(), Error> {
    net::serve_forever()
}
