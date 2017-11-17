use mio;

use errors::*;
use parse;

pub fn serve_forever() -> Result<()> {
    let poll = mio::Poll::new()?;
    let mut events = mio::Events::with_capacity(1024);

    let bind_addresses = &["[::1]:6953", "127.0.0.1:6953"];

    let sockets = bind_addresses
        .iter()
        .enumerate()
        .map(|(id, addr)| -> Result<mio::net::UdpSocket> {
            let socket = mio::net::UdpSocket::bind(&addr.parse()?)?;
            poll.register(
                &socket,
                mio::Token(id),
                mio::Ready::readable(),
                mio::PollOpt::edge(),
            )?;
            Ok(socket)
        })
        .collect::<Result<Vec<mio::net::UdpSocket>>>()?;

    loop {
        poll.poll(&mut events, None)?;
        for event in &events {
            let id: usize = event.token().into();
            if id < sockets.len() {
                let socket = &sockets[id];
                let mut buf = [0u8; 520];
                let (amt, whom) = socket.recv_from(&mut buf)?;
                println!("[{:?}]: {:?}", whom, parse::parse(&buf[..amt])?);
            }
        }
    }
}
