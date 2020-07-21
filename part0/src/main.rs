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

fn timestamp() -> i64 {
let start = SystemTime::now();
let since_the_epoch = start
    .duration_since(UNIX_EPOCH)
    .expect("Time went backwards");
let ms = since_the_epoch.as_secs() as i64 * 1000i64 + (since_the_epoch.subsec_nanos() as f64 / 1_000_000.0) as i64;
ms
}
fn main() {
    let config = include_str!(concat!(env!("PWD"), "/config"));
    let config = config.split("\n");
    let config: Vec<&str> = config.collect(); 
    let server_address = config[2];
    let client_address = config[3];
    let sp_address = config[4];

    println!("attestation start");
    attestation(client_address, sp_address);
    println!("attestation end");
    let syslib = tvm_runtime::SystemLibModule::default();
    let graph_json = include_str!(concat!(env!("OUT_DIR"), "/graph.json"));
    let params_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/params.bin"));
    let params = tvm_runtime::load_param_dict(params_bytes).unwrap();
     
    let graph = tvm_runtime::Graph::try_from(graph_json).unwrap();
    let mut exec = tvm_runtime::GraphExecutor::new(graph, &syslib).unwrap();
    exec.load_params(params);

    let flag = match client_address{
        "None" => false,
        _  => true,
    };
    
    println!("start client");
    let mut socket = TcpStream::connect(client_address).unwrap();
    let mut entropy = entropy_new();
    let mut rng = CtrDrbg::new(&mut entropy, None).unwrap();
    let mut cert = Certificate::from_pem(keys::PEM_CERT).unwrap();
    let mut config = Config::new(Endpoint::Client, Transport::Stream, Preset::Default);
    config.set_rng(Some(&mut rng));
    config.set_ca_list(Some(&mut *cert), None);
    let mut ctx = Context::new(&config).unwrap();
    let mut client_session = ctx.establish(&mut socket, None).unwrap();
    
    
    let listener = TcpListener::bind(server_address).unwrap();
    let mut sy_time = SystemTime::now();
    let mut duration:u128 = 1;
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
        sy_time = SystemTime::now();
        exec.run();
        duration = SystemTime::now().duration_since(sy_time).unwrap().as_micros();
        if flag{
            client_session.write(exec.get_output(0).unwrap().data().as_slice());
        }
        break;

    }
    println!("{:?}", duration);
 }
 