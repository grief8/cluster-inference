// This is the source file of scheduler.
pub struct Slave<'a> {
    pub busy_flag: bool,
    pub slave_ip: &'a str, 
} 

impl<'a> Clone for Slave<'a> {
    fn clone(&self) -> Self {
        Self { busy_flag: self.busy_flag, slave_ip: self.slave_ip }
    }
}
pub struct User<'a> {
    pub sub_model: Vec<&'a str>,
    pub user_ip: &'a str, 
}
impl<'a> Clone for User<'a> {
    fn clone(&self) -> Self {
        let mut sb: Vec<&'a str> = vec![];
        for iter in &self.sub_model{
            sb.push(iter);
        }
        Self { sub_model: sb, user_ip: self.user_ip }
    }
}
pub struct Scheduler<'a> {
    pub map_table: Vec<(&'a str, &'a str, &'a str)>,
    // addresses of users
    pub user_queue: Vec<User<'a>>,
    // addresses of slaves
    pub slave_queue: Vec<Slave<'a>>,
}
impl<'a> Scheduler<'a> {
    fn clone(&self) -> Self {
        Self { map_table: self.map_table.clone(), user_queue: self.user_queue.clone(), slave_queue: self.slave_queue.clone()}
    }
    // Initialize the mapping table.
    // Maybe there is an easier way for configuraration loading.
    pub fn init(self) -> Scheduler<'a> {
        let map_table: Vec<(&str, &str, &str)> = vec![("resnet18", "0,1,2,3,4", "4242-4246")];
        let user_queue: Vec<User<'a>> = vec![];
        let slave_queue: Vec<Slave<'a>> = vec![Slave{busy_flag: false, slave_ip: "127.0.0.1:4242"}, Slave{busy_flag: false, slave_ip: "127.0.0.1:4243"}, 
        Slave{busy_flag: false, slave_ip: "127.0.0.1:4244"}, Slave{busy_flag: false, slave_ip: "127.0.0.1:4245"}, Slave{busy_flag: false, slave_ip: "127.0.0.1:4246"}];
        // let config = include_str!(concat!(env!("PWD"), "/config"));
        // let config = config.split("\n");
        // let config: Vec<&str> = config.collect(); 
        // for model in config{
        //     map_table.push(mod);
        // }
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
    pub fn add_user(&mut self, user_ip: &'a str, model: &str) {
        // let mut sub_model: Vec<&'a str> = vec![];
        // let user_ip = user_ip;
        // let mut user = User{sub_model, user_ip};
        for md in self.map_table.clone(){
            if md.0 == model{
                let sub_model = md.0.split(',').collect();
                let user = User{sub_model, user_ip};
                self.user_queue.push(user);
                println!("{:?}", model);
            }
        }
        // Ok(())
        // Err("failed to init a user")
    }
    pub fn send2user(&mut self, user_ip: &str, slave_ip: &str){

    }
    pub fn change_slave_flag(&mut self, slave_ip: &str){
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
    pub fn find_idle_slave(&mut self, target_model: &str, user_ip: &str) {
        for i in 0..self.user_queue.len(){
            let usr = self.user_queue[i].clone();
            let sc = self.clone();
            let (slv, result) = sc.is_slave_busy(usr.sub_model[0]).unwrap();
            if result {
                continue;
            }
            self.send2user(usr.user_ip, slv.slave_ip);
            self.change_slave_flag(slv.slave_ip);
            self.user_queue[i].sub_model.pop();
        }

    }
    pub fn user_success(mut self, slave_ip: &str){

        self.change_slave_flag(slave_ip);
    }
}