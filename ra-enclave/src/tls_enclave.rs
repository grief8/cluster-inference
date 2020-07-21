use byteorder::{NetworkEndian, WriteBytesExt};
use ra_common::tcp::tcp_accept;
use crate::context::EnclaveRaContext;
use sgx_crypto::random::Rng;
use sgx_crypto::tls_psk::server;
use std::io::Write;
use mbedtls::ssl::Session;

pub const SP_VKEY_PEM: &str = "\
-----BEGIN RSA PUBLIC KEY-----\n
MIIBCgKCAQEAvtc94gzwX0KeL1HJVh6XdHPXXA4PYE+ClqWUvxp5ts1/nLQzJVcy\
1SHMGaPUCr+IZJBeWapkFpgnJnw7YzdQ2kA8k6GiN/k8hlQMWXA2nE0LDeOHX8i7\
fc31lWy5nHdAXj7SfC/YV5RC/yhkJ2cYNMB15VPRHGQRukdVmvHUFunxwfkHq5mM\
xWWAWO5Km490NCWP7CqBH6ezGm5jUhzYT/n5y5EaVpqwKVE1uYA//L4dFSE7aDzD\
CDb50B9uqPaEyKHwc2taLiSPvQjDQE3BpKTDOqsVnojd9br1vYW/uemYnnlOJbSr\
L7pYuPODmV02by5r+7hgXFQkTADwFQBCmwIDAQAB\n\
-----END RSA PUBLIC KEY-----\
\0";


pub fn attestation(client:&str, sp:&str, keep_message:fn(Session)){
    let mut client_stream = tcp_accept(client).expect("Enclave: Client connection failed");
    eprintln!("Enclave: connected to client.");
    let encontext = EnclaveRaContext::init(SP_VKEY_PEM).unwrap();
    let (_signing_key, master_key) = encontext.do_attestation(&mut client_stream).unwrap();

    // talk to SP directly from now on
    //let sp_port = 1235;
    let mut sp_stream = tcp_accept(sp).expect("Enclave: SP connection failed");
    // establish TLS-PSK with SP; enclave is the server
    let mut psk_callback = server::callback(&master_key);
    let mut rng = Rng::new();
    let config = server::config(&mut rng, &mut psk_callback);
    let mut ctx = server::context(&config).unwrap();
    // begin secure communication
    let  mut session = ctx.establish(&mut sp_stream, None).unwrap();
    let msg = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Pellentesque non placerat risus, et lobortis quam. Mauris velit lorem, elementum id neque a, aliquet tempus turpis. Nam eu congue urna, in semper quam. Ut tristique gravida nunc nec feugiat. Proin tincidunt massa a arcu volutpat, sagittis dignissim velit convallis. Cras ac finibus lorem, nec congue felis. Pellentesque fermentum vitae ipsum sed gravida. Nulla consectetur sit amet erat a pellentesque. Donec non velit sem. Sed eu metus felis. Nullam efficitur consequat ante, ut commodo nisi pharetra consequat. Ut accumsan eget ligula laoreet dictum. Maecenas tristique porta convallis. Suspendisse tempor sodales velit, ac luctus urna varius eu. Ut ultrices urna vestibulum vestibulum euismod. Vivamus eu sapien urna.";
    session
        .write_u32::<NetworkEndian>(msg.len() as u32)
        .unwrap();
    write!(&mut session, "{}", msg).unwrap();
    eprintln!("Enclave: done!");
    keep_message(session);
}
