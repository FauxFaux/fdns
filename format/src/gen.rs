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
    pub question: Question,
    pub ttl: u32,
    pub data: Vec<u8>,
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

    pub fn build(&self) -> Result<Vec<u8>> {
        const HEADER_LEN: usize = 12;
        const QUESTION_LEN: usize = 2 + 2 + 2;
        const RR_LEN: usize = QUESTION_LEN + 4 + 2 + 4;

        let mut dat = Vec::with_capacity(
            HEADER_LEN
                + QUESTION_LEN
                + RR_LEN * (self.answers.len() + self.authorities.len() + self.additionals.len()),
        );

        self.append(&mut dat)?;

        Ok(dat)
    }

    pub fn append(&self, dat: &mut Vec<u8>) -> Result<()> {
        dat.write_u16::<BigEndian>(self.transaction_id)?;
        dat.write_u16::<BigEndian>(self.flags)?;
        dat.write_u16::<BigEndian>(match self.question {
            Some(_) => 1,
            None => 0,
        })?;
        dat.write_u16::<BigEndian>(u16(self.answers.len())?)?;
        dat.write_u16::<BigEndian>(u16(self.authorities.len())?)?;
        dat.write_u16::<BigEndian>(u16(self.additionals.len())?)?;

        if let Some(ref question) = self.question {
            write_question(dat, question)?;
        }

        for rr in &self.answers {
            write_rr(dat, rr)?;
        }

        for rr in &self.authorities {
            write_rr(dat, rr)?;
        }

        for rr in &self.additionals {
            write_rr(dat, rr)?;
        }

        Ok(())
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

fn write_question(dat: &mut Vec<u8>, question: &Question) -> Result<()> {
    write_label(dat, &question.label)?;
    dat.write_u16::<BigEndian>(question.req_type.into())?;
    dat.write_u16::<BigEndian>(question.req_class.into())?;
    Ok(())
}

fn write_rr(dat: &mut Vec<u8>, rr: &Rr) -> Result<()> {
    write_question(dat, &rr.question)?;
    dat.write_u32::<BigEndian>(rr.ttl)?;
    dat.write_u16::<BigEndian>(u16(rr.data.len())?)?;
    dat.extend(&rr.data);
    Ok(())
}
