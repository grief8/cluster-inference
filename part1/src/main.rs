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

use std::net::TcpListener;
use byteorder::{NetworkEndian, WriteBytesExt};                                                                                              
//use std::io::Write;
use ra_enclave::tls_enclave::attestation;
use mbedtls::pk::Pk;
use mbedtls::rng::CtrDrbg;
use mbedtls::ssl::config::{Endpoint, Preset, Transport};
use mbedtls::ssl::{Config, Context, Session};
use mbedtls::x509::Certificate;
use std::thread;

#[path = "../../support/mod.rs"]
mod support;
use support::entropy::entropy_new;
use support::keys;
//use ra_enclave::tls_enclave;

use std::{
    convert::TryFrom as _,
    io::{Read as _, Write as _},
    time::{SystemTime, UNIX_EPOCH},
};
//  use image::{FilterType, GenericImageView};
//use ndarray::{Array, Array4};

fn timestamp() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let ms = since_the_epoch.as_secs() as i64 * 1000i64 + (since_the_epoch.subsec_nanos() as f64 / 1_000_000.0) as i64;
    ms
}

fn main() {
    let mut thread_vec = vec![];
    let handle = thread::spawn(move ||{
        println!("attestation start");
        //attestation!{7777,1235};
        attestation("127.0.0.1:7777","127.0.0.1:1235",keep_message);
        println!("attestation end");
    });
    thread_vec.push(handle);
    // let handle = thread::spawn(move ||{
    //     do_tvm();
    // });
    // thread_vec.push(handle);
    for handle in thread_vec {
        // Wait for the thread to finish. Returns a result.
        let _ = handle.join().unwrap();
    }
    
 }
 
 pub fn keep_message(sess:Session){
    let mut session = sess;
    // let msg = "enclave macro!";
    // session.write_u32::<NetworkEndian>(msg.len() as u32).unwrap();
    // write!(&mut session, "{}", msg).unwrap();
 }

pub fn do_tvm(){
    let config = include_str!(concat!(env!("PWD"), "/config"));
    let config = config.split("\n");
    let config: Vec<&str> = config.collect(); 
    let server_address = config[2];
    //let client_address = config[3];
    let syslib = tvm_runtime::SystemLibModule::default();
    let graph_json = include_str!(concat!(env!("OUT_DIR"), "/graph.json"));
    let params_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/params.bin"));
    let params = tvm_runtime::load_param_dict(params_bytes).unwrap();
     
    let graph = tvm_runtime::Graph::try_from(graph_json).unwrap();
    let mut exec = tvm_runtime::GraphExecutor::new(graph, &syslib).unwrap();
    exec.load_params(params);

    let listener = TcpListener::bind(server_address).unwrap();
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut entropy = entropy_new();
        let mut rng = CtrDrbg::new(&mut entropy, None).unwrap();
        let mut cert = Certificate::from_pem(keys::PEM_CERT).unwrap();
        let mut key = Pk::from_private_key(keys::PEM_KEY, None).unwrap();
        let mut config = Config::new(Endpoint::Server, Transport::Stream, Preset::Default);
        config.set_rng(Some(&mut rng));
        config.push_cert(&mut *cert, &mut key).unwrap();
        let mut ctx = Context::new(&config).unwrap();
        let mut server_session = ctx.establish(&mut stream, None).unwrap();
        println!("server_session connect!");
        if let Err(_) =
            server_session.read(exec.get_input("input").unwrap().data().view().as_mut_slice())
        {
            continue;
        }
        let ts1 = timestamp();
        println!("TimeStamp: {}", ts1);
        let sy_time = SystemTime::now();
        exec.run();
        let duration = SystemTime::now().duration_since(sy_time).unwrap().as_micros();
        server_session.write(exec.get_output(0).unwrap().data().as_slice()).unwrap();
        println!("{:?}", duration);
        //only try once
        break;
    }
 }

