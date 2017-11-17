use nom::be_u8;
use nom::be_u16;
use nom::be_u32;
use nom::IResult;

use errors::*;
use usize_from;

#[derive(Debug)]
pub struct Packet<'a> {
    transaction_id: u16,
    flags: u16,
    questions: Vec<Question<'a>>,
    answers: Vec<Rr<'a>>,
    authorities: Vec<Rr<'a>>,
    additionals: Vec<Rr<'a>>,
}

#[derive(Debug)]
pub struct Question<'a> {
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

fn is_end_byte(val: &[u8]) -> bool {
    0 == val[0] || val[0] > 63
}

named!(label<&[u8], &[u8]>,
    recognize!(many_till!(
        length_bytes!(be_u8),
        verify!(take!(1), is_end_byte)
    )));

named!(question<&[u8], Question>, do_parse!(
    label:     label >>
    req_type:  be_u16 >>
    req_class: be_u16 >>
    ( Question { label, req_type, req_class } )
));

named!(rr<&[u8], Rr>, do_parse!(
    label:     label >>
    req_type:  be_u16 >>
    req_class: be_u16 >>
    ttl:       be_u32 >>
    data:      length_bytes!(be_u16) >>
    ( Rr {
        ttl,
        data,
        question: Question { label, req_type, req_class, },
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
        IResult::Done(rem, packet) => {
            if rem.is_empty() {
                Ok(packet)
            } else {
                bail!("unxepected trailing data: {:?}", rem)
            }
        },
        other => bail!("parse error: {:?}", other),
    }
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
