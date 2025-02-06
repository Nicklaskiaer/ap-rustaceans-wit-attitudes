use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashSet;
use std::{fs, thread};
use std::collections::HashMap;
use std::time::Duration;
use wg_2024::config::Config;
use wg_2024::controller::DroneCommand;
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Fragment, Packet};

use crate::simulation_controller::simulation_controller::{simulation_controller_main, SimulationController};
use crate::types::my_drone::MyDrone;
use crate::client::client::{Client, ClientTrait};
use crate::server::server::{ContentServer, Server};

pub fn main() {
    // let current_path = env::current_dir().expect("Unable to get current directory");
    // println!("Current path: {:?}", current_path);
    let config = parse_config("src/config.toml");

    // check for errors in the toml
    // check_toml_validity(&config);

    // hashmap with all packet_channels
    let mut packet_channels: HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)> = HashMap::new();
    for drone in config.drone.iter() {
        packet_channels.insert(drone.id, unbounded());
    }
    for client in config.client.iter() {
        packet_channels.insert(client.id, unbounded());
    }
    for server in config.server.iter() {
        packet_channels.insert(server.id, unbounded());
    }

    // INITIALIZE DRONES
    let (node_event_send_drone, node_event_recv_drone) = unbounded();
    let mut controller_drones = HashMap::new();
    for drone in config.drone.into_iter() {
        // controller
        let (controller_drone_send, controller_drone_recv) = unbounded();
        controller_drones.insert(drone.id, (controller_drone_send, drone.connected_node_ids.clone()));
        let node_event_send_drone = node_event_send_drone.clone();

        // packet
        let packet_recv: Receiver<Packet> = packet_channels[&drone.id].1.clone();
        let packet_send: HashMap<NodeId, Sender<Packet>> = drone
            .connected_node_ids
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        // spawn
        thread::spawn(move || {
            let mut drone = MyDrone::new(
                drone.id,
                node_event_send_drone,
                controller_drone_recv,
                packet_recv,
                packet_send,
                drone.pdr,
            );

            drone.run();
        });
    }

    // INITIALIZE CLIENTS
    let (node_event_send_client, node_event_recv_client) = unbounded();
    let mut controller_clients = HashMap::new();
    for client in config.client.into_iter() {

        // controller
        let (controller_client_send, controller_client_recv) = unbounded();
        controller_clients.insert(client.id, (controller_client_send, client.connected_drone_ids.clone()));
        let node_event_send_client = node_event_send_client.clone();

        // packet
        let packet_recv: Receiver<Packet> = packet_channels[&client.id].1.clone();
        let packet_send: HashMap<NodeId, Sender<Packet>> = client
            .connected_drone_ids.clone()
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        // spawn
        let (assembler_send, assembler_recv) = unbounded();
        thread::spawn(move || {
            let mut client = Client::new(
                client.id,
                client.connected_drone_ids,
                node_event_send_client,
                controller_client_recv,
                packet_send,
                packet_recv,
                vec![],
                HashSet::new(),
                assembler_send,
                assembler_recv
            );

            client.run();
        });
    }

    // INITIALIZE SERVERS
    let (node_event_send_server, node_event_recv_server) = unbounded();
    let mut controller_servers: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for server in config.server.into_iter() {

        // controller
        let (controller_server_send, controller_server_recv) = unbounded();
        controller_servers.insert(server.id, server.connected_drone_ids.clone());
        let node_event_send_server = node_event_send_server.clone();

        // packet
        let packet_recv: Receiver<Packet> = packet_channels[&server.id].1.clone();
        let packet_send: HashMap<NodeId, Sender<Packet>> = server
            .connected_drone_ids.clone()
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        // spawn
        let (assembler_send, assembler_recv) = unbounded();
        thread::spawn(move || {
            let mut server = ContentServer::new(
                server.id,
                server.connected_drone_ids,
                node_event_send_server,
                controller_server_recv,
                packet_send,
                packet_recv,
                vec![],
                HashSet::new(),
                assembler_send,
                assembler_recv
            );

            server.run();
        });
    }


    // INITIALIZE SIMULATION CONTROLLER AND GUI
    let sc = SimulationController::new(
        controller_drones.clone(),
        controller_clients.clone(),
        controller_servers.clone(),
        node_event_recv_drone.clone(),
        node_event_recv_client.clone(),
        node_event_recv_server.clone()
    );
    simulation_controller_main(sc).expect("GUI panicked!");
}

fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).unwrap();
    toml::from_str(&file_str).unwrap()
}


fn check_toml_validity(config: &Config){
    let mut all_ids = HashSet::new();
    let mut drone_ids = HashSet::new();
    let mut client_ids = HashSet::new();
    let mut server_ids = HashSet::new();

    // <editor-fold desc="Do all drones, servers and clients have unique ids?">
    for drone in &config.drone {
        if !drone_ids.insert(drone.id) {
            panic!("found repetition of id: {}!", drone.id)
        }
        if !all_ids.insert(drone.id) {
            panic!("found repetition of id across drones, clients, and servers: {}!", drone.id);
        }
    }

    for client in &config.client {
        if !client_ids.insert(client.id) {
            panic!("found repetition of id: {}!", client.id)
        }
        if !all_ids.insert(client.id) {
            panic!("found repetition of id across drones, clients, and servers: {}!", client.id);
        }
    }

    for server in &config.server {
        if !server_ids.insert(server.id) {
            panic!("found repetition of id: {}!", server.id)
        }
        if !all_ids.insert(server.id) {
            panic!("found repetition of id across drones, clients, and servers: {}!", server.id);
        }
    }
    // </editor-fold>

    // <editor-fold desc="Drone, PDR and connected_node_ids">
    let min_pdr = 0.00;
    let max_pdr = 1.00;
    for drone in &config.drone {
        // do drones have all unique connected_node_ids without their id and with no repetition?
        let mut c_drones_ids = HashSet::new();
        for connected_drone in &drone.connected_node_ids{
            if !connected_drone == drone.id {
                panic!("the drone {} has its id in connected_node_ids!", drone.id)
            }
            if !c_drones_ids.insert(connected_drone){
                panic!("the drone {} has id repetition in connected_node_ids!", drone.id)
            }
        }

        // do drones have a pdr between 0.05% and 5%?
        if !(drone.pdr >= min_pdr) && !(drone.pdr <= max_pdr){
            panic!("{} has an invalid PDR", drone.id)
        }
    }
    // </editor-fold>

    // <editor-fold desc="Client, n. drones connected and connection id repetition">
    let min_drones = 1;
    let max_drones = 2;
    for client in &config.client {
        // do clients have all unique connected_node_ids with no repetition and without any clients or servers id?
        let mut c_client_ids = HashSet::new();
        for connected_drone in &client.connected_drone_ids{
            if !drone_ids.contains(connected_drone) {
                panic!("the client {} has an invalid id in connected_node_ids: {}!", client.id, connected_drone);
            }
            if !c_client_ids.insert(connected_drone) {
                panic!("the client {} has id repetition in connected_node_ids!", client.id)
            }
        }

        // do client the right numbers of drones?
        let n_drones = client.connected_drone_ids.iter().len();
        if !(n_drones >= min_drones) && !(n_drones <= max_drones){
            panic!("{} has an invalid number of drones", client.id)
        }
    }
    // </editor-fold>

    // <editor-fold desc="Server, n. drones connected and connection id repetition">
    let min_drones = 2;
    for server in &config.server {
        // do servers have all unique connected_node_ids with no repetition and without any clients or servers id?
        let mut c_server_ids = HashSet::new();
        for connected_drone in &server.connected_drone_ids{
            if !drone_ids.contains(connected_drone) {
                panic!("the server {} has an invalid id in connected_node_ids: {}!", server.id, connected_drone)
            }
            if !c_server_ids.insert(connected_drone) {
                panic!("the server {} has id repetition in connected_node_ids!", server.id)
            }
        }

        // do client the right numbers of drones?
        let n_drones = server.connected_drone_ids.iter().len();
        if !(n_drones >= min_drones){
            panic!("{} has an invalid number of drones", server.id)
        }
    }
    // </editor-fold>


    // todo: is a bidirectional graph?
}