use crate::types::my_drone::MyDrone;

use std::thread::JoinHandle;
use std::{env, fs, thread};

use wg_2024::config::Config;
use wg_2024::config::Drone as ConfigDrone;
use wg_2024::drone::{Drone, DroneOptions};

pub fn initialize_drones(drones: Vec<ConfigDrone>) -> Vec<JoinHandle<()>> {
    // Vec<Box<dyn Drone>>) -> Vec<JoinHandle<()>>{
    let a = drones[0].id;

    let mut handles = Vec::new();

    for d in drones {
        let handler = thread::spawn(move || {
            let id = d.id;
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

pub fn parse_toml() -> Config {
    let current_path = env::current_dir().expect("Unable to get current directory");
    // println!("Current path: {:?}", current_path);

    let config_data =
        fs::read_to_string("examples/config/config.toml").expect("Unable to read config file");
    let config: Config = toml::from_str(&config_data).expect("Unable to parse TOML");
    //println!("{:#?}", config);
    config
}
