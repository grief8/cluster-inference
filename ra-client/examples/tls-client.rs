extern crate clap;

use byteorder::{NetworkEndian, WriteBytesExt}; 
use ra_client::ClientRaContext;
use ra_common::tcp::tcp_connect;
use std::time::Duration;
use std::io::Write;
use clap::App;

fn main() {
    let matches = App::new("tls-client")
                .version("1.0")
                .author("simplelin. ")
                .about("Does remote attestation")
                .args_from_usage(
                    "-e,  --enclave 'Sets IP and Port for enclave,such as \"127.0.0.1:7777\"'
                    -sp, --service 'Sets IP and Port for service provide,such as \"192.168.1.1:1234\"'")
                .get_matches();
    let enclave  = matches.value_of("enclave").unwrap_or("127.0.0.1:7777");
    let service  = matches.value_of("service").unwrap_or("127.0.0.1:1234");
    //let enclave_ip=enclave[0];
    //let enclave_port = enclave[1].parse::<u16>().unwrap();
    //let sp_ip = service[0];
    //let sp_port = service[1].parse::<u16>().unwrap();
    let timeout = Duration::from_secs(5);

    let mut enclave_stream =
        tcp_connect(enclave, timeout).expect("Client: Enclave connection failed");
    eprintln!("Client: connected to enclave.");

    let mut sp_stream =
        tcp_connect(service, timeout).expect("Client: SP connection failed");
    eprintln!("Client: connected to SP.");

    let context = ClientRaContext::init().unwrap();
    context
        .do_attestation(&mut enclave_stream, &mut sp_stream)
        .unwrap();
    let msg = "127.0.0.1:3333";
    sp_stream.write_u32::<NetworkEndian>(msg.len() as u32).unwrap();
    //write!(&mut sp_stream, "{}", msg).unwrap();
    sp_stream.write(msg.as_bytes()).unwrap();
    eprintln!("Client: done!");
}
