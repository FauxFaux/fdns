use mio;

use errors::*;
use parse;

pub fn serve_forever() -> Result<()> {
    let poll = mio::Poll::new()?;
    let mut events = mio::Events::with_capacity(1024);
    const SERVER: mio::Token = mio::Token(0);

    let socket = mio::net::UdpSocket::bind(&"[::1]:6953".parse()?)?;
    poll.register(
        &socket,
        SERVER,
        mio::Ready::readable(),
        mio::PollOpt::edge(),
    )?;

    loop {
        poll.poll(&mut events, None)?;
        for event in &events {
            match event.token() {
                SERVER => {
                    let mut buf = [0u8; 520];
                    let (amt, whom) = socket.recv_from(&mut buf)?;
                    println!("[{:?}]: {:?}", whom, parse::parse(&buf[..amt])?);
                }
                _ => unreachable!(),
            }
        }
    }
}
