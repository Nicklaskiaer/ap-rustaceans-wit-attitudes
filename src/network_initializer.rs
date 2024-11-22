use crate::config::Config;
use internal::config::Drone as ConfigDrone;
use std::thread::JoinHandle;
use std::{env, fs, thread};
use crossbeam_channel::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use internal::controller::Command;
use crate::drone::drone_usage::MyDrone;
use crate::drone::{Drone, DroneOptions};

pub struct DroneHandle {
    pub id: u8,
    pub controller: Sender<Command>,
    pub thread_handle: JoinHandle<()>,
}


pub fn main(){
    let c = parse_toml();
    let drone_handles = initialize_drones(c.drone);

    // Now you can send commands to specific drones:
    drone_handles[0].controller.send(Command::Crash)
        .expect("Failed to send command to drone");

    // You can also find a specific drone by ID:
    if let Some(drone) = drone_handles.iter().find(|d| d.id == 1) {
        drone.controller.send(Command::Crash)
            .expect("Failed to send command to drone 1");
    }

    // Wait for all drone threads to complete if needed
    for handle in drone_handles {
        handle.thread_handle.join().expect("Drone thread panicked");
    }

    // drone_threads[0].thread().send(Command::Crash).expect("Failed to send command");
    //
    //
    // let copied_drone_test = drone_threads[0].thread().clone();
    // copied_drone_test.send(Command::Crash).expect("Failed to send command");

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
        let controller = sim_contr_send.clone();

        let handler = thread::spawn(move || {
            let mut drone = MyDrone::new(DroneOptions {
                id: d.id as u8,
                sim_contr_recv,
                sim_contr_send,
                packet_recv,
                pdr: d.pdr as f32,
            });

            drone.run();
        });

        handles.push(DroneHandle {
            id: d.id as u8,
            controller,
            thread_handle: handler,
        });
    }

    handles
}


fn parse_toml()->Config{
    let current_path = env::current_dir().expect("Unable to get current directory");
    // println!("Current path: {:?}", current_path);

    let config_data = fs::read_to_string("src/config.toml").expect("Unable to read config file");
    let config: Config = toml::from_str(&config_data).expect("Unable to parse TOML");
    //println!("{:#?}", config);
    config
}




