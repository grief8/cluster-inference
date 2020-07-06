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
 use mbedtls::rng::CtrDrbg;
 use mbedtls::ssl::config::{Endpoint, Preset, Transport};
 use mbedtls::ssl::{Config, Context};
 use mbedtls::x509::Certificate;
 use mbedtls::Result as TlsResult;
 use std::net::{TcpListener, TcpStream};
 
 #[path = "../../support/mod.rs"]
 mod support;
 use support::entropy::entropy_new;
 use support::keys;
 
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
    let shape = config[1].split(",");
    let vec: Vec<&str> = shape.collect();
    let shape = (vec[0].to_string().parse::<usize>().unwrap(), vec[1].to_string().parse::<usize>().unwrap(), vec[2].to_string().parse::<usize>().unwrap(), vec[3].to_string().parse::<usize>().unwrap());

    let syslib = tvm_runtime::SystemLibModule::default();
    let graph_json = include_str!(concat!(env!("OUT_DIR"), "/graph.json"));
    let params_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/params.bin"));
    let params = tvm_runtime::load_param_dict(params_bytes).unwrap();
     
    let graph = tvm_runtime::Graph::try_from(graph_json).unwrap();
    let mut exec = tvm_runtime::GraphExecutor::new(graph, &syslib).unwrap();
    exec.load_params(params);

    let mut rng =rand::thread_rng();
    let mut ran = vec![];
    for _i in 0..shape.0*shape.1*shape.2*shape.3{
        ran.push(rng.gen::<f32>()*256.);
    }
    let x = Array::from_shape_vec(shape, ran).unwrap();
    
    let sy_time = SystemTime::now();
    exec.set_input("input", x.into());
    exec.run();

    let ts1 = timestamp();
    println!("{}", ts1);
    let flag = match client_address{
        "None" => false,
        _  => true,
    };
    if flag{
        // transition 
        let mut socket = std::net::TcpStream::connect(client_address).unwrap();
        let mut entropy = entropy_new();
        let mut rng = CtrDrbg::new(&mut entropy, None).unwrap();
        let mut cert = Certificate::from_pem(keys::PEM_CERT).unwrap();
        let mut config = Config::new(Endpoint::Client, Transport::Stream, Preset::Default);
        config.set_rng(Some(&mut rng));
        config.set_ca_list(Some(&mut *cert), None);
        let mut ctx = Context::new(&config).unwrap();
        let mut client_session = ctx.establish(&mut socket, None).unwrap();
        // println!("client_session connect!");

        
        client_session.write(exec.get_output(0).unwrap().data().as_slice());
    }
    
    let duration = SystemTime::now().duration_since(sy_time).unwrap().as_micros();
    println!("{:?}", duration);
    
    // println!("The index: {:?}", argmax);
    // println!("{:?}", sy_time.elapsed().unwrap().as_micros());
    // println!("{:#?}", output.data().as_slice());

 }
 