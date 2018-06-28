use std::fmt;

use cast::usize;

use failure::Error;
use failure::ResultExt;
use nom::be_u16;
use nom::be_u32;
use nom::IResult;
use nom::Needed;

use OpCode;
use RCode;
use RrClass;
use RrType;

pub struct Packet<'a> {
    raw: &'a [u8],
    decoded: DecodedPacket<'a>,
}

#[derive(Debug)]
pub struct DecodedPacket<'a> {
    pub transaction_id: u16,
    pub flags: u16,
    pub questions: Vec<Question<'a>>,
    pub answers: Vec<Rr<'a>>,
    pub authorities: Vec<Rr<'a>>,
    pub additionals: Vec<Rr<'a>>,
}

#[derive(Debug)]
pub struct Question<'a> {
    pub label: &'a [u8],
    pub req_type: RrType,
    pub req_class: RrClass,
}

#[derive(Debug)]
pub struct Rr<'a> {
    pub question: Question<'a>,
    pub ttl: u32,
    pub data: &'a [u8],
}

impl<'a> Packet<'a> {
    pub fn is_query(&self) -> bool {
        has_bit(self.flags, 15)
    }

    pub fn is_authoritative(&self) -> bool {
        has_bit(self.flags, 10)
    }

    pub fn is_truncated(&self) -> bool {
        has_bit(self.flags, 9)
    }

    pub fn is_recursion_desired(&self) -> bool {
        has_bit(self.flags, 8)
    }

    pub fn is_recursion_available(&self) -> bool {
        has_bit(self.flags, 7)
    }

    pub fn opcode(&self) -> OpCode {
        use OpCode::*;
        match (self.flags << 11) & 0b1111 {
            0 => Query,
            1 => IQuery,
            2 => Status,
            _ => Unknown,
        }
    }

    pub fn rcode(&self) -> RCode {
        use RCode::*;
        match self.flags & 0b1111 {
            0 => NoError,
            1 => FormatError,
            2 => ServerFail,
            3 => NxDomain,
            4 => NotImplemented,
            5 => Refused,
            _ => NotImplemented,
        }
    }

    pub fn reserved_bits_are_zero(&self) -> bool {
        !has_bit(self.flags, 6) && !has_bit(self.flags, 5) && !has_bit(self.flags, 4)
    }

    pub fn decode_label(&self, label: &[u8]) -> Result<Vec<u8>, Error> {
        decode_label(label, self.raw)
    }
}

// not proud of this; it's mostly working around nom being unable to
// capture the entire input, and me being too lazy to copy-out the entire
// Packet constructor twice
impl<'a> ::std::ops::Deref for Packet<'a> {
    type Target = DecodedPacket<'a>;

    fn deref(&self) -> &Self::Target {
        &self.decoded
    }
}

impl<'a> fmt::Debug for Packet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let d = &self.decoded;
        write!(f, "tx:{:04x} ", d.transaction_id)?;
        write!(f, "op: {:?} status: {:?} ", self.opcode(), self.rcode())?;
        write!(f, "flags: ")?;
        if self.is_query() {
            write!(f, "qr ")?;
        }
        if self.is_authoritative() {
            write!(f, "au ")?;
        }
        if self.is_truncated() {
            write!(f, "tr ")?;
        }
        if self.is_recursion_desired() {
            write!(f, "rd ")?;
        }
        if self.is_recursion_available() {
            write!(f, "ra ")?;
        }

        for q in &self.questions {
            write!(
                f,
                "q: {} ty: {:?} cl: {:?}; ",
                String::from_utf8_lossy(&self.decode_label(q.label).unwrap()),
                q.req_type,
                q.req_class
            )?;
        }

        write!(
            f,
            "ans: {}, auth: {}, add: {}",
            self.answers.len(),
            self.authorities.len(),
            self.additionals.len()
        )?;

        Ok(())
    }
}

pub fn decode_label(label: &[u8], packet: &[u8]) -> Result<Vec<u8>, Error> {
    let mut pos = 0;
    let mut ret = Vec::with_capacity(label.len());
    loop {
        ensure!(pos < label.len(), "out of bounds read");
        let len = usize::from(label[pos]);
        pos += 1;

        if 0 == len {
            break;
        }

        if len < 64 {
            ret.extend(&label[pos..pos + len]);
            ret.push(b'.');
            pos += len;
        } else {
            let off = (len & 0b0011_1111) * 0x10 + usize(label[pos]);
            ret.extend(
                decode_label(&packet[off..], packet)
                    .with_context(|_| format_err!("processing {:?}", label))?,
            );
            break;
        }
    }

    Ok(ret)
}

fn label(from: &[u8]) -> IResult<&[u8], &[u8]> {
    let mut pos = 0;
    loop {
        if pos >= from.len() {
            return IResult::Incomplete(Needed::Size(pos));
        }

        let len = usize::from(from[pos]);

        pos += 1;

        if 0 == len {
            break;
        }

        if len < 64 {
            pos += len;
        } else {
            pos += 1;
            break;
        }
    }

    IResult::Done(&from[pos..], &from[..pos])
}

named!(question<&[u8], Question>, do_parse!(
    label:     label >>
    req_type:  be_u16 >>
    req_class: be_u16 >>
    ( Question {
        label,
        req_type: req_type.into(),
        req_class: req_class.into(),
    } )
));

named!(rr<&[u8], Rr>, do_parse!(
    question:  question >>
    ttl:       be_u32 >>
    data:      length_bytes!(be_u16) >>
    ( Rr {
        question,
        ttl,
        data,
    } )
));

named!(record<&[u8], DecodedPacket>, do_parse!(
    transaction_id: be_u16 >>
    flags:          be_u16 >>
    questions:      be_u16 >>
    answers:        be_u16 >>
    authorities:    be_u16 >>
    additionals:    be_u16 >>
    questions:      count!(question, usize(questions))   >>
    answers:        count!(rr,       usize(answers))     >>
    authorities:    count!(rr,       usize(authorities)) >>
    additionals:    count!(rr,       usize(additionals)) >>
    (DecodedPacket {
        transaction_id, flags,
        questions, answers, authorities, additionals,
    })
));

pub fn parse(data: &[u8]) -> Result<Packet, Error> {
    match record(data) {
        IResult::Done(rem, decoded) => if rem.is_empty() {
            Ok(Packet { raw: data, decoded })
        } else {
            bail!("unexpected trailing data: {:?}", rem)
        },
        other => bail!("parse error: {:?}", other),
    }
}

#[inline]
fn has_bit(flags: u16, bit: u8) -> bool {
    assert!(bit < 16);
    (flags & (1 << bit)) == (1 << bit)
}

#[cfg(test)]
mod tests {
    use super::decode_label;
    use super::label;
    use super::parse;

    #[test]
    fn packet_a_fau_xxx() {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        const DATA: [u8; 36] = [
            0x8e, 0xe1, 0x01, 0x20, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x03, 0x66, 0x61, 0x75,
            0x03, 0x78, 0x78, 0x78, 0x00, 0x00, 0x01, 0x00,
            0x01, 0x00, 0x00, 0x29, 0x10, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ];

        let packet = parse(&DATA).unwrap();

        assert_eq!(1, packet.questions.len());
        assert_eq!(0, packet.answers.len());
        assert_eq!(0, packet.authorities.len());
        assert_eq!(1, packet.additionals.len());

        let first_question = &packet.questions[0];
        assert_eq!(b"\x03fau\x03xxx\0", first_question.label);
    }

    #[test]
    fn label_fau_xxx() {
        let exp = b"\x03fau\x03xxx\0";
        let (rem, matched) = label(exp).unwrap();
        assert_eq!(&[0u8; 0], rem);
        assert_eq!(exp, matched);
    }

    #[test]
    fn label_ref() {
        let exp = b"\x03fau\xc0\x0c";
        let (rem, matched) = label(exp).unwrap();
        assert_eq!(&[0u8; 0], rem);
        assert_eq!(exp, matched);
    }

    #[test]
    fn label_only_ref() {
        let exp = b"\xc0\x0c";
        let (rem, matched) = label(exp).unwrap();
        assert_eq!(&[0u8; 0], rem);
        assert_eq!(exp, matched);
    }

    #[test]
    fn gmail_mx() {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let data = [
            /* 0000 */ 0x3d, 0x1f, 0x81, 0x80, 0x00, 0x01, 0x00, 0x05, // =.......
            /* 0008 */ 0x00, 0x00, 0x00, 0x01, 0x05, 0x67, 0x6d, 0x61, // .....gma
            /* 0010 */ 0x69, 0x6c, 0x03, 0x63, 0x6f, 0x6d, 0x00, 0x00, // il.com..
            /* 0018 */ 0x0f, 0x00, 0x01, 0xc0, 0x0c, 0x00, 0x0f, 0x00, // ........
            /* 0020 */ 0x01, 0x00, 0x00, 0x02, 0x2b, 0x00, 0x20, 0x00, // ....+...
            /* 0028 */ 0x1e, 0x04, 0x61, 0x6c, 0x74, 0x33, 0x0d, 0x67, // ..alt3.g
            /* 0030 */ 0x6d, 0x61, 0x69, 0x6c, 0x2d, 0x73, 0x6d, 0x74, // mail-smt
            /* 0038 */ 0x70, 0x2d, 0x69, 0x6e, 0x01, 0x6c, 0x06, 0x67, // p-in.l.g
            /* 0040 */ 0x6f, 0x6f, 0x67, 0x6c, 0x65, 0xc0, 0x12, 0xc0, // oogle...
            /* 0048 */ 0x0c, 0x00, 0x0f, 0x00, 0x01, 0x00, 0x00, 0x02, // ........
            /* 0050 */ 0x2b, 0x00, 0x09, 0x00, 0x14, 0x04, 0x61, 0x6c, // +.....al
            /* 0058 */ 0x74, 0x32, 0xc0, 0x2e, 0xc0, 0x0c, 0x00, 0x0f, // t2......
            /* 0060 */ 0x00, 0x01, 0x00, 0x00, 0x02, 0x2b, 0x00, 0x09, // .....+..
            /* 0068 */ 0x00, 0x28, 0x04, 0x61, 0x6c, 0x74, 0x34, 0xc0, // .(.alt4.
            /* 0070 */ 0x2e, 0xc0, 0x0c, 0x00, 0x0f, 0x00, 0x01, 0x00, // ........
            /* 0078 */ 0x00, 0x02, 0x2b, 0x00, 0x09, 0x00, 0x0a, 0x04, // ..+.....
            /* 0080 */ 0x61, 0x6c, 0x74, 0x31, 0xc0, 0x2e, 0xc0, 0x0c, // alt1....
            /* 0088 */ 0x00, 0x0f, 0x00, 0x01, 0x00, 0x00, 0x02, 0x2b, // .......+
            /* 0090 */ 0x00, 0x04, 0x00, 0x05, 0xc0, 0x2e, 0x00, 0x00, // ........
            /* 0098 */ 0x29, 0xff, 0xd6, 0x00, 0x00, 0x00, 0x00, 0x00, // ).......
            /* 00a0 */ 0x00,                                           // .
        ];

        let packet = parse(&data).unwrap();

        assert_eq!(1, packet.questions.len());
        assert_eq!(5, packet.answers.len());

        assert_eq!(
            b"gmail.com.",
            packet
                .decode_label(packet.answers[0].question.label)
                .unwrap()
                .as_slice()
        );

        // MX record, so two bytes of priority before the label
        assert_eq!(
            b"alt4.gmail-smtp-in.l.google.com.",
            decode_label(&packet.answers[2].data[2..], &data)
                .unwrap()
                .as_slice()
        );
    }
}
