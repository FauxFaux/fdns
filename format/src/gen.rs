use byteorder::BigEndian;
use byteorder::WriteBytesExt;
use cast::u16;
use cast::u8;

use errors::*;
use RCode;
use RrClass;
use RrType;

#[derive(Clone, Debug, Default)]
pub struct Builder {
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
        const RESPONSE: u16 = 1 << 15;
        Builder {
            transaction_id,
            flags: RESPONSE,
            ..Builder::default()
        }
    }

    pub fn error(&mut self, code: RCode) -> &mut Builder {
        self.flags = (self.flags & !0b1111) | u16(code.mask());
        self
    }

    pub fn set_query(&mut self, question: Option<Question>) -> &mut Builder {
        self.question = question;
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
        dat.write_u16::<BigEndian>(u16(self.answers.len()).unwrap())
            .unwrap();
        dat.write_u16::<BigEndian>(u16(self.authorities.len()).unwrap())
            .unwrap();
        dat.write_u16::<BigEndian>(u16(self.additionals.len()).unwrap())
            .unwrap();

        // TODO: ..the actual data
        if let Some(ref question) = self.question {
            write_label(&mut dat, &question.label)
                .expect("label validation can actually fail here...");
            dat.write_u16::<BigEndian>(question.req_type.into())
                .expect("writing to vec");
            dat.write_u16::<BigEndian>(question.req_class.into())
                .expect("writing to vec");
        }
        assert_eq!(0, self.answers.len());
        assert_eq!(0, self.authorities.len());
        assert_eq!(0, self.additionals.len());

        dat
    }
}

impl Question {
    pub fn new(label: String, req_type: RrType, req_class: RrClass) -> Question {
        Question {
            label,
            req_type,
            req_class,
        }
    }
}

fn write_label<S: AsRef<str>>(dest: &mut Vec<u8>, label: S) -> Result<()> {
    for part in label.as_ref().split('.') {
        dest.push(u8(part.len())?);
        dest.extend(part.bytes());
    }
    assert_eq!(0, dest[dest.len() - 1]);
    Ok(())
}
