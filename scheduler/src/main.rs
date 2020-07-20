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
 extern crate image;
 extern crate ndarray;
 extern crate rand;
 extern crate mbedtls;

use std::net::{TcpListener, TcpStream};
use mbedtls::rng::Rdrand;
use mbedtls::pk::Pk;
use mbedtls::rng::CtrDrbg;
use mbedtls::ssl::config::{Endpoint, Preset, Transport};
use mbedtls::ssl::{Config, Context, Session};
use mbedtls::x509::Certificate;
use mbedtls::Result as TlsResult;
use std::fmt::Write;

#[path = "../../support/mod.rs"]
mod support;
use support::entropy::entropy_new;
use support::keys;
use ra_enclave::tls_enclave::attestation;

use rand::Rng;
use std::{
    convert::TryFrom as _,
    io::{Read as _, Write as _},
    time::{SystemTime, UNIX_EPOCH},
};
//  use image::{FilterType, GenericImageView};
use ndarray::{Array, Array4};
use std::{thread, time};
use std::sync::{Arc, Mutex};
mod master;
use master::{Scheduler, User};
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
    let config = config.split("\n");
    let config: Vec<&str> = config.collect(); 
    let server_address = config[2];
    let server_address = config[2];
    let client_address = config[3];
    let attestation_port = config[4];

    println!("attestation start");
    attestation(attestation_port.to_string().parse::<u16>().unwrap());
    println!("attestation end");

    let listener = TcpListener::bind(server_address).unwrap();

    let mut scheduler = Arc::new(Mutex::new(Scheduler{map_table: vec![], user_queue: vec![], slave_queue: vec![]}.init()));
    // let mut queue = Arc::new(Mutex::new(vec![]));
    // let (tx, rx) = mpsc::channel();
    let mut thread_vec: Vec<thread::JoinHandle<()>> = Vec::new();
    
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
                let msg = match std::string::String::from_utf8(array) {
                    Ok(v) => v,
                    Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                };
                let msg = msg.strip_suffix('\n').unwrap();
                // println!("{:?}", msg);
                // Different measures according to value of msg.
                if msg == ""{
                    break;
                }
                else if msg.starts_with("resnet18") || msg.starts_with("mobilenetv1"){
                    // init_user(msg);
                    model = msg.to_string();
                    scheduler.add_user("localhost:2222", model);
                    let uq = scheduler.user_queue.clone();
                    // for user in uq{
                    //     println!("sb: {:?}", user.sub_model);
                    // }
                    println!("{:?}", uq.len());
                }
                else if &msg[..1] >= "0" && &msg[..1] <= "9"{
                // else if msg.starts_with("wabibabo"){
                    // println!("num: {:?}", msg);
                    let message: Vec<&str> = msg.split(',').collect();
                    scheduler.change_slave_flag(slv_ip.as_str());
                    let ip = scheduler.apply4slave("localhost:2222", message[1].to_string());
                    slv_ip = ip.to_string();
                    server_session.write(ip.as_bytes()).unwrap();
                    println!("catch: {:?}", ip);
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
 