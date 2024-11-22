use std::{env, fs, thread};
use std::thread::JoinHandle;
use crossbeam_channel::{Receiver, Sender};

use wg_2024::drone::{Drone, DroneOptions};
use wg_2024::config::{Config, Drone as ConfigDrone};
use wg_2024::controller::Command;

use crate::types::my_drone::MyDrone;

pub struct DroneHandle {
    pub id: u8,
    pub simulation_controller: (Sender<Command>, Receiver<Command>),
    pub thread_handle: JoinHandle<()>,
}

pub fn main(){
    let c = parse_toml();
    let drone_handles = initialize_drones(c.drone);
    
    // find a drone by ID and send it a Command:
    let drone_id = 1;
    if let Some(drone) = drone_handles.iter().find(|d| d.id == drone_id) {
        drone.simulation_controller.0.send(Command::Crash)
            .expect(&format!("Failed to send command to drone {}", drone_id));
    }

    // Join all drone threads to ensure they complete before exiting
    for drone_handle in drone_handles {
        drone_handle.thread_handle.join().expect("Failed to join drone thread");
    }
    
    // let cloned_handler = drone_threads[0].clone();
    // handles.push((handler, cloned_handler));

    // let servers_threads = initialize_servers(&c.drone);
    // let clients_threads = initialize_clients(&c.drone);
    // fn topology_setup(&drone_threads, &servers_threads, &clients_threads)
    // fn crash_handle()
}

fn initialize_drones(drones: Vec<ConfigDrone>) -> Vec<DroneHandle>{
    let mut handles = Vec::new();

    for d in drones {

        let (sim_contr_send, sim_contr_recv) = crossbeam_channel::unbounded();
        let (packet_send, packet_recv) = crossbeam_channel::unbounded();

        let simulation_controller = (sim_contr_send.clone(), sim_contr_recv.clone());

        let handler:JoinHandle<()> = thread::spawn(move || {
            let mut drone = MyDrone::new(DroneOptions {
                id: d.id as u8,
                sim_contr_recv,
                sim_contr_send,
                packet_recv,
                pdr: d.pdr,
            });

            drone.run();
        });

        handles.push(DroneHandle {
            id: d.id as u8,
            simulation_controller,
            thread_handle: handler,
        });
    }

    handles
}


fn parse_toml()->Config{
    // let current_path = env::current_dir().expect("Unable to get current directory");
    // println!("Current path: {:?}", current_path);

    let path = "src/config.toml";
    let config_data = fs::read_to_string(path).expect("Unable to read config file");
    let config: Config = toml::from_str(&config_data).expect("Unable to parse TOML");
    //println!("{:#?}", config);
    config
}
