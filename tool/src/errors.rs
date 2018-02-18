error_chain! {
    links {
        Parse(::fdns_parse::Error, ::fdns_parse::ErrorKind);
    }

    foreign_links {
        NetAddrParse(::std::net::AddrParseError);
        Io(::std::io::Error);
    }
}
