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

pub struct slave{
    pub busy_flag: bool,
    pub slave_ip: &str, 
} 
pub struct user{
    pub sub_model: Vec<&str>,
    pub user_ip: &str, 
}
pub #[derive(Debug)]
struct scheduler {
    pub mut map_table: Vec<(&str, &str, &str)>,
    // addresses of users
    pub mut user_queue: Vec<user>,
    // addresses of slaves
    pub mut slave_queue: Vec<slave>,
}
impl scheduler {
    // Initialize the mapping table.
    // Maybe there is an easier way for configuraration loading.
    pub fn init() -> scheduler {
        let mut map_table: Vec<(&str, &str, &str)> = vec![];
        pub user_queue: Vec<&str>;
        pub slave_queue: Vec<&str>;
        let config = include_str!(concat!(env!("PWD"), "/config"));
        let config = config.split("\n");
        let config: Vec<&str> = config.collect(); 
        for model in config{
            map_table.push(model);
        }
        scheduler{map_table, user_queue, user_ip}
    }
    pub fn is_slave_busy(self, slave_ip) -> (slave, bool){
        for slv in self.slave_queue{
            if(slv.slave_ip == slave_ip){
                match slv.busy_flag {
                    true => (slv, true),
                    false => (slv, false),
                    None => (slv, true),
                }
            }
        }
        (slv, true)
    }
    pub fn send2user(self, user_ip, slave_ip){
        self.user_queue.
    }
    pub change_slave_flag(self, slave_ip){
        for i in 0..self.user_queue.len(){
            if(self.user_queue[i].slave_ip == slave_ip)
            {
                self.user_queue[i].busy_flag = match self.user_queue[i].busy_flag{
                    false => true,
                    true => false,
                };
            }
        }
    }
    pub fn find_idle_slave(self, &mut target_model, &mut user_ip) {
        for i in 0..self.user_queue.len(){
            let usr = self.user_queue[i];
            let result = is_slave_busy(usr.sub_model.top());
            if(result.1){
                continue;
            }
            send2user(usr.user_ip, slv.slave_ip);
            change_slave_flag(slv.slave_ip);
            self.user_queue[i].sub_model.pop();
        }

    }
    pub fn user_success(self, slave_ip){
        change_slave_flag(slave_ip);
    }
}