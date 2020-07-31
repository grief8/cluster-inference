use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use ra_common::tcp::{tcp_connect};
use ra_sp::{SpConfigs, SpRaContext, AttestationResult, HttpRespWrap};
use sgx_crypto::random::{entropy_new, Rng};
use sgx_crypto::key_exchange::DHKE;
use sgx_crypto::tls_psk::client;
use sgx_crypto::aes_gcm::AESGCM;
use std::io::{Read, Write};
use std::{mem, thread};
use std::sync::{Mutex, Arc};
use std::time::Duration;
use mbedtls::ssl::Session;
use std::net::{TcpListener, TcpStream};
use std::collections::HashMap;
use hyper::{Response};

const SCHEDULE_ID: u8 = 255;
const STOP_LABEL: u8 = 250;
fn parse_config_file(path: &str) -> SpConfigs {
    serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap()
}

fn main() {
    let configs = parse_config_file("examples/data/settings.json");
    let listener_address = configs.listener_address.clone();
    let http_report =  Arc::new(Mutex::new(HashMap::new()));
    let listener = TcpListener::bind(listener_address).unwrap();
    //listener.set_nonblocking(true).expect("Cannot set non-blocking");
    eprintln!("SP: starting to listen client.");
    for stream in listener.incoming() {
        eprintln!("SP: new socket incoming.");
        match stream {
            Err(e) => {
                eprintln!("failed: {}", e);
                continue;
            },
            Ok(mut stream) => {
                let enclave_id = stream.read_u8().unwrap();
                let client_ip = stream.peer_addr().unwrap().ip();
                let spconfig = match configs.generate_spconfig(enclave_id, &client_ip) {
                    Err(e) => {
                        eprintln!("client err, ip:{}", stream.peer_addr().unwrap());
                        eprintln!("the client socket is not configed in settings.json: {}", e);
                        return;
                    },
                    Ok(config) =>config
                };
                let http_report = Arc::clone(&http_report);
                thread::spawn(move||{
                    
                    let mut entropy = entropy_new();
                    let enclave_port = spconfig.enclave_port.clone();
                    let mut context = SpRaContext::init(spconfig, &mut entropy).unwrap();
                    let result = match context.do_attestation(&mut stream){
                        Ok(result)=>{
                            let http_resp = mem::replace(&mut context.get_ias_client().http_resp, None).unwrap();
                            http_report.lock().unwrap().insert(context.get_spconfig().enclave_id, http_resp);
                            println!("enclave_id: {}", context.get_spconfig().enclave_id);
                            result
                        },
                        Err(e)=>{
                            println!("Do remote attestation failed, the reason is: {}", e);
                            return;
                        }
                    };
                    if context.get_spconfig().enclave_id == SCHEDULE_ID{
                        send_http_report_to_schedule(&mut http_report.lock().unwrap(), enclave_port);
                        return;
                    }else {
                        // establish TLS-PSK with enclave; SP is the client
                        do_tls_psk(result, enclave_port, keep_message);
                    }
                });
                // thread_vec.push(handle);
            }
        }
    }
}
// establish TLS-PSK with enclave; SP is the client
pub fn do_tls_psk(result:AttestationResult, enclave_port: String, keep_message:fn(Session)){
    let timeout = Duration::from_secs(5);
    let mut enclave_stream =
        tcp_connect(&enclave_port, timeout).expect("SP: Enclave connection failed");
    let mut entropy = entropy_new();
    let mut rng = Rng::new(&mut entropy).unwrap();
    let config = client::config(&mut rng, &result.master_key).unwrap();
    let mut ctx = client::context(&config).unwrap();

    // begin secure communication
    let mut session = ctx.establish(&mut enclave_stream, None).unwrap();

    keep_message(session);
 }
//enable enclave can communite with sp for more message
pub fn keep_message(session:Session){
    let mut sess = session;
    let len = sess.read_u32::<NetworkEndian>().unwrap() as usize;
    let mut msg = vec![0u8; len];
    sess.read_exact(&mut msg[..]).unwrap();
    let msg = std::str::from_utf8(msg.as_slice()).unwrap();
    let msg_ref = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Pellentesque non placerat risus, et lobortis quam. Mauris velit lorem, elementum id neque a, aliquet tempus turpis. Nam eu congue urna, in semper quam. Ut tristique gravida nunc nec feugiat. Proin tincidunt massa a arcu volutpat, sagittis dignissim velit convallis. Cras ac finibus lorem, nec congue felis. Pellentesque fermentum vitae ipsum sed gravida. Nulla consectetur sit amet erat a pellentesque. Donec non velit sem. Sed eu metus felis. Nullam efficitur consequat ante, ut commodo nisi pharetra consequat. Ut accumsan eget ligula laoreet dictum. Maecenas tristique porta convallis. Suspendisse tempor sodales velit, ac luctus urna varius eu. Ut ultrices urna vestibulum vestibulum euismod. Vivamus eu sapien urna.";
    assert_eq!(msg, msg_ref);
    eprintln!("SP: message from Enclave = \"{}\"", msg);
    eprintln!("SP: done!");
 }
//send all of the report getting from IAS to schedule for check
pub fn send_http_report_to_schedule(http_report:&mut HashMap<u8, Response<Vec<u8>>>, enclave_port: String){
    let timeout = Duration::from_secs(50);
    println!("{}",enclave_port);
    let mut enclave_stream =
        tcp_connect(&enclave_port, timeout).expect("SP: Enclave connection failed");
    for (k, v) in http_report{
        enclave_stream.write_u8(*k).unwrap();
        let wrapped = HttpRespWrap{
            map: v.headers().clone(),
            statu:v.status(),
        };
        let header = serde_json::to_vec(&wrapped).unwrap();
        let len = header.len() as u32;
        enclave_stream.write_u32::<NetworkEndian>(len).unwrap();
        enclave_stream.write_all(&header).unwrap();
        let body = v.body_mut();
        let len = body.len() as u32;
        enclave_stream.write_u32::<NetworkEndian>(len).unwrap();
        enclave_stream.write_all(&body).unwrap();
    }
    enclave_stream.write_u8(STOP_LABEL).unwrap();
}
