use byteorder::ByteOrder;
use byteorder::BigEndian;
use byteorder::WriteBytesExt;
use cast::u16;

use RCode;
use RrClass;
use RrType;


#[derive(Clone, Debug, Default)]
struct Builder {
    transaction_id: u16,
    flags: u16,
    question: Option<Question>,
    answers: Vec<Rr>,
    authorities: Vec<Rr>,
    additionals: Vec<Rr>,
}

#[derive(Clone, Debug)]
pub struct Question {
    label: String,
    req_type: RrType,
    req_class: RrClass,
}

#[derive(Clone, Debug)]
pub struct Rr {
    question: Question,
    ttl: u32,
    data: Vec<u8>,
}

impl Builder {
    pub fn response_to(transaction_id: u16) -> Builder {
        Builder {
            transaction_id,
            .. Builder::default()
        }
    }

    pub fn error(&mut self, code: RCode) -> &mut Builder {
        self.flags = (self.flags & !0b1111) | u16(code.mask());
        self
    }

    pub fn add_answer(&mut self, rr: Rr) -> &mut Builder {
        self.answers.push(rr);
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let mut dat = Vec::with_capacity(12);
        dat.write_u16::<BigEndian>(self.transaction_id).unwrap();
        dat.write_u16::<BigEndian>(self.flags).unwrap();
        dat.write_u16::<BigEndian>(match self.question {
            Some(_) => 1,
            None => 0,
        }).unwrap();
        dat.write_u16::<BigEndian>(u16(self.answers.len()).unwrap()).unwrap();
        dat.write_u16::<BigEndian>(u16(self.authorities.len()).unwrap()).unwrap();
        dat.write_u16::<BigEndian>(u16(self.additionals.len()).unwrap()).unwrap();

        // TODO: ..the actual data
        assert!(self.question.is_none());
        assert_eq!(0, self.answers.len());
        assert_eq!(0, self.authorities.len());
        assert_eq!(0, self.additionals.len());

        dat
    }
}
