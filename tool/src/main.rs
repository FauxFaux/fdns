mod net;

use anyhow::Result;

fn main() -> Result<()> {
    net::serve_forever()
}
