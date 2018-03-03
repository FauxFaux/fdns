extern crate byteorder;
extern crate cast;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate nom;

pub mod gen;
mod errors;
pub mod parse;

pub use errors::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OpCode {
    Query,
    IQuery,
    Status,
    Unknown,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RCode {
    NoError,
    FormatError,
    ServerFail,
    NxDomain,
    NotImplemented,
    Refused,
    Unknown,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RrClass {
    Internet,
    Unknown(u16),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RrType {
    A,
    Aaaa,
    Ns,
    CName,
    Mx,
    Txt,
    Srv,
    Unknown(u16),
}

impl From<u8> for RCode {
    fn from(bits: u8) -> Self {
        use RCode::*;
        match bits & 0b1111 {
            0 => NoError,
            1 => FormatError,
            2 => ServerFail,
            3 => NxDomain,
            4 => NotImplemented,
            5 => Refused,
            _ => NotImplemented,
        }
    }
}

impl RCode {
    fn mask(&self) -> u8 {
        use RCode::*;
        match *self {
            NoError => 0,
            FormatError => 1,
            ServerFail => 2,
            NxDomain => 3,
            Unknown | NotImplemented => 4,
            Refused => 5,
        }
    }
}

impl From<u16> for RrClass {
    fn from(be: u16) -> Self {
        match be {
            1 => RrClass::Internet,
            other => RrClass::Unknown(other),
        }
    }
}

impl From<RrClass> for u16 {
    fn from(be: RrClass) -> Self {
        match be {
            RrClass::Internet => 1,
            RrClass::Unknown(other) => other,
        }
    }
}

impl From<u16> for RrType {
    fn from(be: u16) -> RrType {
        use self::RrType::*;
        match be {
            1 => A,
            28 => Aaaa,
            2 => Ns,
            5 => CName,
            15 => Mx,
            16 => Txt,
            33 => Srv,
            other => Unknown(other),
        }
    }
}

impl From<RrType> for u16 {
    fn from(rr: RrType) -> u16 {
        use self::RrType::*;
        match rr {
            A => 1,
            Aaaa => 28,
            Ns => 2,
            CName => 5,
            Mx => 15,
            Txt => 16,
            Srv => 33,
            Unknown(other) => other,
        }
    }
}
