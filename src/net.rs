use byteorder::ByteOrder;
use byteorder::BigEndian;

use mio;
use mio::net::UdpSocket;

use errors::*;
use parse;

pub fn serve_forever() -> Result<()> {
    let poll = mio::Poll::new()?;
    let mut events = mio::Events::with_capacity(1024);

    let bind_addresses = &["[::1]:6953", "127.0.0.1:6953"];

    let sockets = bind_addresses
        .iter()
        .enumerate()
        .map(|(id, addr)| -> Result<UdpSocket> {
            let socket = UdpSocket::bind(&addr.parse()?)?;
            poll.register(
                &socket,
                mio::Token(id),
                mio::Ready::readable(),
                mio::PollOpt::edge(),
            )?;
            Ok(socket)
        })
        .collect::<Result<Vec<UdpSocket>>>()?;

    loop {
        poll.poll(&mut events, None)?;
        for event in &events {
            let id: usize = event.token().into();
            if id < sockets.len() {
                let socket: &UdpSocket = &sockets[id];
                let mut buf = [0u8; 520];
                let (amt, whom) = socket.recv_from(&mut buf)?;
                let parsed = parse::parse(&buf[..amt])?;

                socket.send_to(
                    &short_reply(
                        parsed.transaction_id,
                        parsed.opcode(),
                        false,
                        false,
                        true,
                        parse::RCode::ServerFail,
                    ),
                    &whom,
                )?;
            }
        }
    }
}

fn short_reply(
    id: u16,
    opcode: parse::OpCode,
    authoritative: bool,
    truncated: bool,
    recursion_available: bool,
    rcode: parse::RCode,
) -> [u8; 12] {
    let mut ret = [0u8; 12];
    BigEndian::write_u16(&mut ret, id);
    ret[2] |= 0b1000_0000;
    ret[3] |= 0b10;
    ret
}
