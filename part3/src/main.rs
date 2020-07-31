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
use byteorder::{NetworkEndian, WriteBytesExt, ReadBytesExt};                                                                                              
use sgx_crypto::{
    key_exchange::DHKE,
    tls_psk::client,
    aes_gcm::AESGCM,
    random::Rng,
    signature::SigningKey,
};
use ra_enclave::tls_enclave::attestation;
use mbedtls::pk::Pk;
use mbedtls::rng::CtrDrbg;
use mbedtls::ssl::config::{Endpoint, Preset, Transport};
use mbedtls::ssl::{Config, Context, Session};
use mbedtls::x509::Certificate;

#[path = "../../support/mod.rs"]
mod support;
use support::entropy::entropy_new;
use support::keys;

use std::{
    convert::TryFrom as _,
    io::{Read as _, Write as _},
    time::{SystemTime, UNIX_EPOCH},
    thread,
};
use serde_json::{Result, Value};
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
    // let 
    let handle = thread::spawn(move ||{
        println!("attestation start");
        let config = include_str!(concat!(env!("PWD"), "/config"));
        let config: Value = serde_json::from_str(config).unwrap();
        let client_address = config["client_address"].as_str().unwrap();
        let sp_address = config["sp_address"].as_str().unwrap();
        let mut sign_key = attestation(client_address, sp_address, keep_message).unwrap();
        println!("attestation end");
        do_tvm(&mut sign_key);
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
 
pub fn keep_message(session:Session){
    let mut sess = session;
    let msg = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Pellentesque non placerat risus, et lobortis quam. Mauris velit lorem, elementum id neque a, aliquet tempus turpis. Nam eu congue urna, in semper quam. Ut tristique gravida nunc nec feugiat. Proin tincidunt massa a arcu volutpat, sagittis dignissim velit convallis. Cras ac finibus lorem, nec congue felis. Pellentesque fermentum vitae ipsum sed gravida. Nulla consectetur sit amet erat a pellentesque. Donec non velit sem. Sed eu metus felis. Nullam efficitur consequat ante, ut commodo nisi pharetra consequat. Ut accumsan eget ligula laoreet dictum. Maecenas tristique porta convallis. Suspendisse tempor sodales velit, ac luctus urna varius eu. Ut ultrices urna vestibulum vestibulum euismod. Vivamus eu sapien urna.";
    sess
        .write_u32::<NetworkEndian>(msg.len() as u32)
        .unwrap();
    write!(&mut sess, "{}", msg).unwrap();
}

pub fn do_tvm(sign_key: &mut SigningKey){
    let config = include_str!(concat!(env!("PWD"), "/config"));
    let config: Value = serde_json::from_str(config).unwrap();
    let server_address = config["server_address"].as_str().unwrap();
    //let client_address = config[3];
    let syslib = tvm_runtime::SystemLibModule::default();
    let graph_json = include_str!(concat!(env!("OUT_DIR"), "/graph.json"));
    let params_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/params.bin"));
    let params = tvm_runtime::load_param_dict(params_bytes).unwrap();
     
    let graph = tvm_runtime::Graph::try_from(graph_json).unwrap();
    let mut exec = tvm_runtime::GraphExecutor::new(graph, &syslib).unwrap();
    exec.load_params(params);
    let listener = TcpListener::bind(server_address).unwrap();
    println!("addr: {}", server_address);
    for stream in listener.incoming() {
        println!("server_session connect!");
        let mut server_session = stream.unwrap();
        let mut rng = Rng::new();
        let mut nonce = vec![0u8;16];
        server_session.read_exact(&mut nonce[..]).unwrap();
        let sign_mess = sign_key.ecdsa_sign(&nonce, &mut rng).unwrap();
        let len = sign_mess.len() as u32;
        server_session.write_u32::<NetworkEndian>(len).unwrap();
        server_session.write_all(&sign_mess).unwrap();

        let dh_key = DHKE::generate_keypair(&mut rng).expect("generate ecdh key pair failed!");
        let dh_public = dh_key.get_public_key().expect("get ecdh public key failed!");
        let len  = server_session.read_u32::<NetworkEndian>().unwrap() as usize;
        println!("read ecdh g_b: {:?}", len);
        let mut g_b = vec![0u8; len];
        server_session.read_exact(&mut g_b[..]).unwrap();
        let len = dh_public.len() as u32;
        server_session.write_u32::<NetworkEndian>(len).unwrap();
        server_session.write_all(&dh_public).unwrap();
        let aes_key = dh_key.derive_key_len(&mut g_b,&mut rng, 32 as usize).expect("derive aes key!");
        let len  = server_session.read_u32::<NetworkEndian>().unwrap() as usize;
        let mut message = vec![0u8; len];
        server_session.read_exact(&mut message[..]).unwrap();
        let mut tag = message.get(len-12..len).unwrap().to_vec();
        let mut message = message.get(0..len-12).unwrap();
        let mut plain = vec![0u8;message.len()];
        let mut aes_ctx=AESGCM::new_with_key(&aes_key, aes_key.len() as u32).expect("new_with_key");
        aes_ctx.decrypt(&message, &mut plain[..], &mut tag).expect("aes gcm decrypt");
        exec.get_input("input").unwrap().data().view().as_mut_slice()[..].clone_from_slice(&plain);
        let ts1 = timestamp();
        println!("TimeStamp: {}", ts1);
        let sy_time = SystemTime::now();
        exec.run();
        let duration = SystemTime::now().duration_since(sy_time).unwrap().as_micros();
        let mut plain = exec.get_output(0).unwrap().data().as_slice();
        let mut cipher = vec![0u8;plain.len()];
        let mut tag = [0u8;12];
        println!("start encrypt");
        let len = aes_ctx.encrypt(plain, &mut cipher[..], &mut tag[..]).expect("aes gcm encrypt");
        cipher.truncate(len);
        server_session.write_u32::<NetworkEndian>(len as u32 +12).unwrap();
        cipher.extend_from_slice(&tag);
        println!("start write_all");
        server_session.write_all(&cipher);
        println!("{:?}", duration);
        //only try once
        break;
    }
 }

