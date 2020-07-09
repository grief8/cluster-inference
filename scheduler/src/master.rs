// This is the source file of scheduler.
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
use rand::Rng;
use std::{
   convert::TryFrom as _,
   io::{Read as _, Write as _},
   time::{SystemTime, UNIX_EPOCH},
};
//  use image::{FilterType, GenericImageView};
use ndarray::{Array, Array4};
use std::{thread, time};

pub struct Slave<'a> {
    pub busy_flag: bool,
    pub slave_ip: &'a str, 
} 
pub struct User<'a> {
    pub sub_model: Vec<&'a str>,
    pub user_ip: &'a str, 
}
pub struct Scheduler<'a> {
    pub map_table: Vec<(&'a str, &'a str, &'a str)>,
    // addresses of users
    pub user_queue: Vec<User<'a>>,
    // addresses of slaves
    pub  slave_queue: Vec<Slave<'a>>,
}
impl<'a> Scheduler<'a> {
    // Initialize the mapping table.
    // Maybe there is an easier way for configuraration loading.
    pub fn init(self) -> Scheduler<'a> {
        let mut map_table: Vec<(&str, &str, &str)> = vec![];
        let user_queue: Vec<User<'a>> = vec![];
        let slave_queue: Vec<Slave<'a>> = vec![];
        let config = include_str!(concat!(env!("PWD"), "/config"));
        let config = config.split("\n");
        let config: Vec<&str> = config.collect(); 
        for model in config{
            map_table.push(mod-el);
        }
        Scheduler {map_table, user_queue, slave_queue }
    }
    pub fn is_slave_busy(self, slave_ip: &str) -> Result<(Slave, bool), bool>{
        for slv in self.slave_queue{
            if slv.slave_ip == slave_ip {
                let result = match slv.busy_flag {
                    true => Ok((slv, true)),
                    false => Ok((slv, false)),
                    _ => Err(true),
                };
            }
        }
        Err(true)
    }
    pub fn send2user(self, user_ip: &str, slave_ip: &str){

    }
    pub fn change_slave_flag(self, slave_ip: &str){
        for i in 0..self.slave_queue.len(){
            if self.slave_queue[i].slave_ip == slave_ip
            {
                self.slave_queue[i].busy_flag = match self.slave_queue[i].busy_flag{
                    false => true,
                    true => false,
                };
            }
        }
    }
    pub fn find_idle_slave(self, target_model: &str, user_ip: &str) {
        for i in 0..self.user_queue.len(){
            let usr = self.user_queue[i];
            let (slv, result) = self.is_slave_busy(usr.sub_model[0]).unwrap();
            if result {
                continue;
            }
            self.send2user(usr.user_ip, slv.slave_ip);
            self.change_slave_flag(slv.slave_ip);
            self.user_queue[i].sub_model.pop();
        }

    }
    pub fn user_success(self, slave_ip: &str){

        self.change_slave_flag(slave_ip);
    }
}
fn handle_client(mut stream: TcpStream) -> &'static [u8]{
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
    server_session.read(exec.get_input("input").unwrap().data().view().as_mut_slice());
    exec.get_output(0).unwrap().data().as_slice()
    // if flag{
    //     client_session.write(exec.get_output(0).unwrap().data().as_slice());
    // }
} 
fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    let mut thread_vec: Vec<thread::JoinHandle<()>> = Vec::new();

    for stream in listener.incoming() {
        // handle_client(stream?);
        let stream = stream.unwrap();
        let handle = thread::spawn(|| {
            handle_client(stream);
        });
        thread_vec.push(handle);
    }

    for handle in thread_vec {
        handle.join().unwrap();
    }
    
    Ok(())
}