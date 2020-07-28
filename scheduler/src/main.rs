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
extern crate ndarray;
extern crate rand;
extern crate byteorder;
extern crate mbedtls;

use byteorder::{NetworkEndian, ReadBytesExt};
use std::net::{TcpListener, TcpStream};                                                                                        
use std::io::{Write, Read};
use ra_enclave::tls_enclave::{attestation_get_report, HttpRespWrap};
use ra_enclave::attestation_response::AttestationResponse;
use sgx_crypto::signature::{VerificationKey};
use sgx_crypto::random::Rng;
use mbedtls::rng::CtrDrbg;
use mbedtls::pk::Pk;
use mbedtls::ssl::config::{Endpoint, Preset, Transport};
use mbedtls::ssl::{Config, Context, Session};
use mbedtls::x509::Certificate;
use mbedtls::Result as TlsResult;

#[path = "../../support/mod.rs"]
mod support;
use support::entropy::entropy_new;
use support::keys;


use std::{
    convert::TryFrom as _,
    io::{Read as _, Write as _},
    time::{SystemTime, UNIX_EPOCH},
};
use serde_json::{Result, Value};
//  use image::{FilterType, GenericImageView};
use ndarray::{Array, Array4};
use std::{thread, time};
use std::sync::{Arc, Mutex};
mod master;
use master::{Scheduler, User, Slave};
fn timestamp() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards"); 
    let ms = since_the_epoch.as_secs() as i64 * 1000i64 + (since_the_epoch.subsec_nanos() as f64 / 1_000_000.0) as i64;
    ms
}

fn main() -> std::io::Result<()> {
    let config = include_str!(concat!(env!("PWD"), "/config"));
    let config: Value = serde_json::from_str(config).unwrap();
    let server_address = config["server_address"].as_str().unwrap();
    let client_address = config["client_address"].as_str().unwrap();
    let attestation_address = config["attestation_address"].as_str().unwrap();
    let sp_address = config["sp_address"].as_str().unwrap();

    let map_table = config.clone();
    let user_queue: Vec<User> = vec![];
    let mut slave_queue: Vec<Slave> = vec![];
    // for i in 0..config["slave_address"]["resnet18"]["slave"].as_object().unwrap().len(){
    //     slave_queue.push(Slave{busy_flag: false, slave_ip: config["slave_address"]["resnet18"]["slave"][i.to_string()].as_str().unwrap()})
    // }
    // for i in 0..config["slave_address"]["mobilenetv1"]["slave"].as_object().unwrap().len(){
    //     slave_queue.push(Slave{busy_flag: false, slave_ip: config["slave_address"]["mobilenetv1"]["slave"][i.to_string()].as_str().unwrap()})
    // }
    let scheduler = Scheduler {map_table: map_table, user_queue, slave_queue }.init(config.clone());
    println!("attestation start");
    let mut sign_key = attestation_get_report(client_address, sp_address, keep_message).unwrap();
    {
        let mut rng = Rng::new();
        let message = [0x1,0x2,0x3,0x4,0x5,0x6,0x7];
        let sign_mess = sign_key.ecdsa_sign(&message, &mut rng).unwrap();
        let public_key = sign_key.get_public_key().unwrap();
        println!("public_key:{:x?}", &public_key);
        let mut verify_key = VerificationKey::new_from_binary(&public_key).expect("get new verify public key failed!");
        verify_key.verify(&message, &sign_mess).expect("verify failed!");
    }
    println!("attestation end");

    let listener = TcpListener::bind(server_address).unwrap();

    let mut mt: Value = serde_json::from_str("{}").unwrap();
    let mut config: &'static str = include_str!(concat!(env!("PWD"), "/config"));
    let mut scheduler = Arc::new(Mutex::new(scheduler));
    // let mut queue = Arc::new(Mutex::new(vec![]));
    // let (tx, rx) = mpsc::channel();
    let mut thread_vec: Vec<thread::JoinHandle<()>> = Vec::new();
    // let serialized = serde_json::to_string(&scheduler).unwrap();
    // println!("serializeds = {}", serialized);
    // let mut sy_time = SystemTime::now();
    // let mut duration: u128 = 1;
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let scheduler = scheduler.clone(); // potential sync errors here
        // let queue = queue.clone();
        // let tx = tx.clone();
        let handle = thread::spawn( move || {
            let mut scheduler = scheduler.lock().unwrap();
            // let mut user = User{sub_model: vec![], user_ip: "",model: ""};
            let mut slv_ip: String = "".to_string();
            let mut model = "".to_string();
            let mut entropy = entropy_new();
            let mut rng = CtrDrbg::new(&mut entropy, None).unwrap();
            let mut cert = Certificate::from_pem(keys::PEM_CERT).unwrap();
            let mut key = Pk::from_private_key(keys::PEM_KEY, None).unwrap();
            let mut config = Config::new(Endpoint::Server, Transport::Stream, Preset::Default);
            config.set_rng(Some(&mut rng));
            config.push_cert(&mut *cert, &mut key).unwrap();
            let mut ctx = Context::new(&config).unwrap();
            let mut server_session = ctx.establish(&mut stream, None).unwrap();
            // println!("server_session connect!");
            loop {
                let mut array: [u8; 256] = [0; 256];
                server_session.read(&mut array).unwrap();
                let mut array = array.to_vec();
                array.retain(|&x| x != 0);
                let mut msg = match std::string::String::from_utf8(array) {
                    Ok(v) => v,
                    Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                };
                if msg.ends_with("\n"){
                    msg = msg.strip_suffix('\n').unwrap().to_string();
                }
                // println!("raw message: {:?}", msg);
                // Different measures according to value of msg.
                // let msg = msg.clone();
                if msg == ""{
                    scheduler.change_slave_flag(slv_ip.clone());
                    break;
                }
                else if msg.starts_with("resnet18") || msg.starts_with("mobilenetv1"){
                    model = msg.to_string();
                    scheduler.add_user(model.clone());
                    let ip = scheduler.apply4slave(model, slv_ip.clone());
                    if ip.starts_with(','){
                        slv_ip = ip.clone().strip_prefix(',').unwrap().to_string();
                    }
                    else{
                        let m = ip.clone();
                        let m: Vec<&str> = m.split(',').collect();
                        slv_ip = m[1].to_string();
                    }
                    server_session.write(format!("{},{}",ip,"key").as_bytes()).unwrap();
                    println!("{}", format!("{},{}",ip,"key"));
                    // let uq = scheduler.user_queue.clone();
                    // for user in uq{
                    //     println!("sb: {:?}", user.sub_model);
                    // }
                    // println!("{:?}", uq.len());
                }
                else
                {
                    println!("{:?}", msg);
                    server_session.write("wrong message!!!".as_bytes()).unwrap();
                }
            }
            
        });
        thread_vec.push(handle);
    }

    for handle in thread_vec {
        handle.join().unwrap();
    }
    // let queue = queue.lock().unwrap();
    // println!("{:?}", queue);
    // println!("{:?}", duration);
    Ok(())   
}
pub fn keep_message(socket: TcpStream){
    let mut sock = socket;
    loop{
        let id = sock.read_u8().unwrap();
        if id == 250u8{
            break;
        }
        println!("receiced id is: {:?}", id);
        let len  = sock.read_u32::<NetworkEndian>().unwrap() as usize;
        println!("httpresp header len: {:?}", len);
        let mut header = vec![0u8; len];
        sock.read_exact(&mut header[..]).unwrap();
        let header: HttpRespWrap = serde_json::from_slice(&header).unwrap();
        let len  = sock.read_u32::<NetworkEndian>().unwrap() as usize;
        let mut body = vec![0u8; len];
        sock.read_exact(&mut body[..]).unwrap();
        
        let attresp = AttestationResponse::from_response(&header.map, body).unwrap();
        // println!("{:?}", attresp.isv_enclave_quote_body);
        let quote = base64::decode(&attresp.isv_enclave_quote_body).unwrap();
        if cfg!(feature = "verbose") {
            println!("\nmr enclave value:");
            for i in &quote[112..144]{
                print!("0x{:>0width$x?}, ", i,width=2);   
            }
            println!("\nmr signer value:");
            for i in &quote[176..208]{
                print!("0x{:>0width$x?}, ", i,width=2);   
            }
            println!("\nsigner public key:");
            for i in &quote[399..432]{
                print!("0x{:>0width$x?}, ", i,width=2);   
            }
            println!();
        }      
    }
}