error_chain! {
    foreign_links {
        Cast(::cast::Error);
        NetAddrParse(::std::net::AddrParseError);
        Io(::std::io::Error);
    }
}
