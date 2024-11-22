use std::{env, fs, thread};
use std::thread::JoinHandle;
use internal::config::Drone as ConfigDrone;
use crate::config::Config;

use crate::drone::drone_usage::MyDrone;
use crate::drone::{Drone, DroneOptions};

fn main(){
    let c = parse_toml();
    let drone_threads = initialize_drones(c.drone);

    // let servers_threads = initialize_servers(&c.drone);
    // let clients_threads = initialize_clients(&c.drone);
    // fn topology_setup(&drone_threads, &servers_threads, &clients_threads)
    // fn crash_handle()
}

fn initialize_drones(drones: Vec<ConfigDrone>) -> Vec<JoinHandle<()>>{

    // Vec<Box<dyn Drone>>) -> Vec<JoinHandle<()>>{
    let a = drones[0].id;

    let mut handles = Vec::new();

    for d in drones {
        let handler = thread::spawn(move || {
            let id = d;
            let (sim_contr_send, sim_contr_recv) = crossbeam_channel::unbounded();
            let (_packet_send, packet_recv) = crossbeam_channel::unbounded();
            let mut drone = MyDrone::new(DroneOptions {
                id,
                sim_contr_recv,
                sim_contr_send,
                packet_recv,
                pdr: 0.1,
            });

            drone.run();
        });

        handles.push(handler);
    }

    handles
}


fn parse_toml()->Config{
    let current_path = env::current_dir().expect("Unable to get current directory");
    // println!("Current path: {:?}", current_path);

    let config_data = fs::read_to_string("examples/config/config.toml").expect("Unable to read config file");
    let config: Config = toml::from_str(&config_data).expect("Unable to parse TOML");
    //println!("{:#?}", config);
    config
}



