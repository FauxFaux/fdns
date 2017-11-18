error_chain! {
    foreign_links {
        NetAddrParse(::std::net::AddrParseError);
        Io(::std::io::Error);
    }
}

#[cfg(intellij_type_hinting)]
pub use error_chain_for_dumb_ides::stubs::*;
