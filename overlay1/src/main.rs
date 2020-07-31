/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

extern crate tvm_runtime;
// extern crate image;
extern crate ndarray;
extern crate rand;
extern crate mbedtls;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use sgx_crypto::{
    key_exchange::DHKE,
    tls_psk::client,
    aes_gcm::AESGCM,
    random::Rng,
    signature::VerificationKey,
};
use std::net::{TcpListener, TcpStream};
use mbedtls::rng::CtrDrbg;
use mbedtls::ssl::config::{Endpoint, Preset, Transport};
use mbedtls::ssl::{Config, Context, Session};
use mbedtls::x509::Certificate;
use serde_json::{Result, Value};

#[path = "../../support/mod.rs"]
mod support;
use support::entropy::entropy_new;
use support::keys;
use ra_enclave::tls_enclave::{attestation_get_report, HttpRespWrap};
use ra_enclave::attestation_response::AttestationResponse;
use rand::Rng as data_rng;
use std::{
    io::{Read as _, Write as _},
    time::{SystemTime, UNIX_EPOCH},
    slice,
};
//  use image::{FilterType, GenericImageView};

fn timestamp() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let ms = since_the_epoch.as_secs() as i64 * 1000i64 + (since_the_epoch.subsec_nanos() as f64 / 1_000_000.0) as i64;
    ms
}

fn gen_input_data(shape: (i32, i32, i32, i32)) -> Vec<f32>{
    let mut rng =rand::thread_rng();
    let mut ran = vec![];
    for _i in 0..shape.0*shape.1*shape.2*shape.3{
        ran.push(rng.gen::<f32>()*256.);
    }
    ran
}

fn launch_slave_session(address: &str, public_key: Vec<u8>, message: &mut [u8]) -> Vec<u8>{
    println!("connecting to {:#?}", address);
    let mut enclave_stream = TcpStream::connect(address).unwrap();
    let mut rng = Rng::new();
    let mut nonce = vec![0u8;16];
    rng.random(&mut nonce[..]).unwrap();
    // println!("nonce:{:?}",&nonce);
    enclave_stream.write_all(&nonce).unwrap();
    let len  = enclave_stream.read_u32::<NetworkEndian>().unwrap() as usize;
    let mut sign_mess = vec![0u8;len];
    enclave_stream.read_exact(&mut sign_mess[..]).unwrap();
    println!("sign_mess:{:?}",&sign_mess);
    let mut verify_key = VerificationKey::new_from_binary(&public_key).expect("get new verify public key failed!");
    verify_key.verify(&nonce, &sign_mess).expect("verify failed!");
    // println!("nonce:{:?}",&nonce);

    let dh_key = DHKE::generate_keypair(&mut rng).expect("generate ecdh key pair failed!");
    let dh_public = dh_key.get_public_key().expect("get ecdh public key failed!");
    let len = dh_public.len() as u32;
    enclave_stream.write_u32::<NetworkEndian>(len).unwrap();
    enclave_stream.write_all(&dh_public).unwrap();
    let len  = enclave_stream.read_u32::<NetworkEndian>().unwrap() as usize;
    // println!("read ga len: {:?}", len);
    let mut g_a = vec![0u8; len];
    enclave_stream.read_exact(&mut g_a[..]).unwrap();
    let aes_key = dh_key.derive_key_len(&g_a,&mut rng, 32 as usize).expect("derive aes key!");
    // println!("aes_key:{:?}",&aes_key);
    // println!("aes_key len:{:?}",aes_key.len() as u32);
    let mut aes_ctx=AESGCM::new_with_key(&aes_key, aes_key.len() as u32).expect("new_with_key");
    let mut cipher = vec![0u8;message.len()];
    let mut tag = [0u8;12];
    // println!("start encrypt");
    let len = aes_ctx.encrypt(message, &mut cipher[..], &mut tag[..]).expect("aes gcm encrypt");
    cipher.truncate(len);
    enclave_stream.write_u32::<NetworkEndian>(len as u32 +12).unwrap();
    cipher.extend_from_slice(&tag);
    enclave_stream.write_all(&cipher);

    let len  = enclave_stream.read_u32::<NetworkEndian>().unwrap() as usize;
    let mut msg = vec![0u8;len];
    enclave_stream.read_exact(&mut msg[..]);
    let mut tag = msg.get(len-12..len).unwrap().to_vec();
    let mut msg = msg.get(0..len-12).unwrap();
    let mut plain = vec![0u8;msg.len()];

    aes_ctx.decrypt(&msg, &mut plain, &mut tag[..]).expect("aes gcm decrypt");
    plain
}
fn main() {
    let config = include_str!(concat!(env!("PWD"), "/config"));
    let config: Value = serde_json::from_str(config).unwrap();
    let client_address = config["client_address"].as_str().unwrap();

    // println!("attestation start");
    // attestation(attestation_port.to_string().parse::<u16>().unwrap(), );
    // println!("attestation end");

    let flag = match client_address{
        "None" => false,
        _  => true,
    };
    
    let mut data= gen_input_data((1, 3, 224, 224));
    let mut usr_data = unsafe{
        slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, data.len() * 4).to_vec()
    };
    // println!("connecting to scheduler {:?}", client_address);
    let mut socket = TcpStream::connect(client_address).unwrap();
    let mut entropy = entropy_new();
    let mut rng = CtrDrbg::new(&mut entropy, None).unwrap();
    let mut cert = Certificate::from_pem(keys::PEM_CERT).unwrap();
    let mut config = Config::new(Endpoint::Client, Transport::Stream, Preset::Default);
    config.set_rng(Some(&mut rng));
    config.set_ca_list(Some(&mut *cert), None);
    let mut ctx = Context::new(&config).unwrap();
    let mut client_session = ctx.establish(&mut socket, None).unwrap();

    let sy_time = SystemTime::now();
    client_session.write("attestation".as_bytes());
    let quote = verify_report(&mut client_session).unwrap();
    println!("attestation time: {:?}", SystemTime::now().duration_since(sy_time).unwrap().as_micros());

    let mut sy_time = SystemTime::now();
    client_session.write("resnet18|127.0.0.1".as_bytes());
    // let usr_data = Vec:new();
    loop{
        let mut array: [u8; 256] = [0; 256]; 
        client_session.read(&mut array).unwrap();
        let mut array = array.to_vec();
        array.retain(|&x| x != 0);
        let mut public_key = array.split_off(array.len()-33);
        let mut msg = match std::string::String::from_utf8(array) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        // println!("msg: {:?}", msg);
        println!("schedule time: {:?}", SystemTime::now().duration_since(sy_time).unwrap().as_micros());
        sy_time = SystemTime::now();
        if msg.ends_with('\n') {
            msg = msg.strip_suffix('\n').unwrap().to_string();
        }
        if msg.starts_with("finished"){
            let message:Vec<&str> = msg.split("|").collect();
            println!("message: {:?}", public_key);
            let mut verify_key = VerificationKey::new_from_binary(&public_key).expect("get new verify public key failed!");
            // println!("verify_key: {:?}", verify_key);
            let mut data = launch_slave_session(message[1], public_key, &mut usr_data);
            usr_data.clear();
            usr_data.append(&mut data);
            // println!("result: {:#?}", data);
            break;
        }
        else {
            let message:Vec<&str> = msg.split("|").collect();
            // println!("message: {:?}", public_key); 
            let mut verify_key = VerificationKey::new_from_binary(&public_key).expect("get new verify public key failed!");
            // let ts1 = timestamp();
            // println!("slave TimeStamp: {}", ts1);
            let mut data = launch_slave_session(message[1], public_key, &mut usr_data);
            usr_data.clear();
            usr_data.append(&mut data);
            println!("usr_data.len: {}", usr_data.len());
            client_session.write("resnet18|127.0.0.1".as_bytes());
        }
    }
    // println!("total time: {:?}", SystemTime::now().duration_since(sy_time).unwrap().as_micros());
 }
 
pub fn verify_report(sock: &mut Session) -> Result<Vec<u8>>{
    let len  = sock.read_u32::<NetworkEndian>().unwrap() as usize;
    let mut header = vec![0u8; len];
    sock.read_exact(&mut header[..]).unwrap();
    let header: HttpRespWrap = serde_json::from_slice(&header).unwrap();
    let len  = sock.read_u32::<NetworkEndian>().unwrap() as usize;
    let mut body = vec![0u8; len];
    sock.read_exact(&mut body[..]).unwrap();
    let sy_time = SystemTime::now();
    let attresp = AttestationResponse::from_response(&header.map, body).unwrap();
    let quote = base64::decode(&attresp.isv_enclave_quote_body).unwrap();
    println!("verification time: {:?}", SystemTime::now().duration_since(sy_time).unwrap().as_micros());
    // if cfg!(feature = "verbose") {
    //     println!("\nmr enclave value:");
    //     for i in &quote[112..144]{
    //         print!("0x{:>0width$x?}, ", i,width=2);   
    //     }
    //     println!("\nmr signer value:");
    //     for i in &quote[176..208]{
    //         print!("0x{:>0width$x?}, ", i,width=2);   
    //     }
    //     println!("\nsigner public key:");
    //     for i in &quote[399..432]{
    //         print!("0x{:>0width$x?}, ", i,width=2);   
    //     }
    //     println!();
    // }
    Ok(quote)  
}