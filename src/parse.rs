use nom::be_u8;
use nom::be_u16;
use nom::be_u32;
use nom::IResult;

use errors::*;
use usize_from;

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
    0 == val[0] || val[0] > 63
}

fn locate(from: &[u8]) -> IResult<&[u8], usize> {
    IResult::Done(from, from.len())
}

named!(label<&[u8], &[u8]>,
    recognize!(many_till!(
        length_bytes!(be_u8),
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
    questions:      count!(question, usize_from(questions))   >>
    answers:        count!(rr,       usize_from(answers))     >>
    authorities:    count!(rr,       usize_from(authorities)) >>
    additionals:    count!(rr,       usize_from(additionals)) >>
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
}
