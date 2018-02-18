use cast::usize;

use nom::be_u16;
use nom::be_u32;
use nom::IResult;

use errors::*;

#[derive(Debug)]
pub struct Packet<'a> {
    pub transaction_id: u16,
    flags: u16,
    questions: Vec<Question<'a>>,
    answers: Vec<Rr<'a>>,
    authorities: Vec<Rr<'a>>,
    additionals: Vec<Rr<'a>>,
}

#[derive(Debug)]
pub struct Question<'a> {
    label_from_end: usize,
    label: &'a [u8],
    req_type: u16,
    req_class: u16,
}

#[derive(Debug)]
pub struct Rr<'a> {
    question: Question<'a>,
    ttl: u32,
    data: &'a [u8],
}

pub enum OpCode {
    Query,
    IQuery,
    Status,
    Unknown,
}

pub enum RCode {
    NoError,
    FormatError,
    ServerFail,
    NxDomain,
    NotImplemented,
    Refused,
    Unknown,
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
        use self::OpCode::*;
        match (self.flags << 11) & 0b1111 {
            0 => Query,
            1 => IQuery,
            2 => Status,
            _ => Unknown,
        }
    }

    pub fn rcode(&self) -> RCode {
        use self::RCode::*;
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
}

fn is_end_byte(val: &[u8]) -> bool {
    0 == val[0]
}

fn locate(from: &[u8]) -> IResult<&[u8], usize> {
    IResult::Done(from, from.len())
}

fn token(from: &[u8]) -> IResult<&[u8], &[u8]> {
    let len = usize::from(from[0]);

    if len < 64 {
        IResult::Done(&from[len + 1..], &from[..len + 1])
    } else {
        IResult::Done(&from[2..], &from[..2])
    }
}

fn label(from: &[u8]) -> IResult<&[u8], &[u8]> {
    let mut pos = 0;
    loop {
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


named!(label_old<&[u8], &[u8]>,
    recognize!(many_till!(
        call!(token),
        verify!(take!(1), is_end_byte)
    )));


named!(question<&[u8], Question>, do_parse!(
    position:  locate >>
    label:     label >>
    req_type:  be_u16 >>
    req_class: be_u16 >>
    ( Question {
        label_from_end: position,
        label,
        req_type,
        req_class
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

named!(record<&[u8], Packet>, do_parse!(
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
    (Packet {
        transaction_id, flags,
        questions, answers, authorities, additionals,
    })
));

pub fn parse(data: &[u8]) -> Result<Packet> {
    match record(data) {
        IResult::Done(rem, packet) => if rem.is_empty() {
            Ok(packet)
        } else {
            bail!("unxepected trailing data: {:?}", rem)
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
    use super::parse;
    use super::label;

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
    }
}
