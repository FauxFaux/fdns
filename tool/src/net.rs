use failure::Error;
use mio;
use mio::net::UdpSocket;

use fdns_format::parse;

pub fn serve_forever() -> Result<(), Error> {
    let poll = mio::Poll::new()?;
    let mut events = mio::Events::with_capacity(1024);

    let bind_addresses = &["[::1]:6953", "127.0.0.1:6953"];

    let sockets = bind_addresses
        .iter()
        .enumerate()
        .map(|(id, addr)| -> Result<UdpSocket, Error> {
            let socket = UdpSocket::bind(&addr.parse()?)?;
            poll.register(
                &socket,
                mio::Token(id),
                mio::Ready::readable(),
                mio::PollOpt::edge(),
            )?;
            Ok(socket)
        })
        .collect::<Result<Vec<UdpSocket>, Error>>()?;

    loop {
        poll.poll(&mut events, None)?;
        for event in &events {
            let id: usize = event.token().into();
            if id >= sockets.len() {
                unreachable!()
            }

            let socket: &UdpSocket = &sockets[id];
            let mut buf = [0u8; 512];
            let (amt, whom) = socket.recv_from(&mut buf)?;
            if amt < 12 {
                println!("[{:?}]: short read", whom);
                continue;
            }

            socket.send_to(
                match handle(&buf[..amt]) {
                    Ok(Handle::ShortReply(r)) => short_reply(&mut buf, true, r),
                    Err(e) => {
                        println!("[{:?}]: error: {:?}", whom, e);
                        short_reply(&mut buf, true, 2)
                    }
                },
                &whom,
            )?;
        }
    }
}

enum Handle {
    ShortReply(u8),
}

fn handle(buf: &[u8]) -> Result<Handle, Error> {
    let parsed = parse::parse(buf)?;
    println!("{:?}", parsed);
    Ok(Handle::ShortReply(5))
}

fn short_reply(buf: &mut [u8], recursion_available: bool, rcode: u8) -> &[u8] {
    assert!(buf.len() >= 12);
    assert!(rcode < 6);

    // response = yes, (opcode, recursion-desired) copied
    buf[2] = 0b1000_0000 | (buf[2] & 0b0111_1000) | (buf[2] & 0b1);

    buf[3] = rcode;
    if recursion_available {
        buf[3] |= 0b1000_0000;
    }

    for i in 4..12 {
        buf[i] = 0;
    }

    &buf[..12]
}
