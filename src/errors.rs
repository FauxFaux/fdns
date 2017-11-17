error_chain! {
    foreign_links {
        NetAddrParse(::std::net::AddrParseError);
        Io(::std::io::Error);
    }
}
