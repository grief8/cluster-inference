use crate::ClientRaContext;
use ra_common::tcp::tcp_connect;
use std::time::Duration;

fn client(enclave_addr: &str, sp_addr: &str) {
    // let enclave_port = 7777;
    // let sp_port = 1234;
    // let localhost = "localhost";
    let timeout = Duration::from_secs(5);

    let mut enclave_stream =
        tcp_connect(enclave_addr, timeout).expect("Client: Enclave connection failed");
    eprintln!("Client: connected to enclave.");

    let mut sp_stream =
        tcp_connect(sp_addr, timeout).expect("Client: SP connection failed");
    eprintln!("Client: connected to SP.");

    let context = ClientRaContext::init().unwrap();
    context
        .do_attestation(&mut enclave_stream, &mut sp_stream)
        .unwrap();
    eprintln!("Client: done!");
}
