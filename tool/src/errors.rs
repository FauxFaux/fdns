error_chain! {
    links {
        Format(::fdns_format::Error, ::fdns_format::ErrorKind);
    }

    foreign_links {
        NetAddrParse(::std::net::AddrParseError);
        Io(::std::io::Error);
    }
}
