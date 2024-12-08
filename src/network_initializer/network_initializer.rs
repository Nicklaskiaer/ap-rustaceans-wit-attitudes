use crossbeam_channel::unbounded;
use std::collections::HashSet;
use std::{fs, thread};

use std::collections::HashMap;
use wg_2024::config::Config;
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, Packet, PacketType};
use crate::simulation_controller::simulation_controller::SimulationController;
use crate::types::my_drone::MyDrone;
use crate::test_fragments;


pub fn main() {
    // let current_path = env::current_dir().expect("Unable to get current directory");
    // println!("Current path: {:?}", current_path);
    let config = parse_config("src/config.toml");

    // check for errors in the toml
    check_toml_validity(&config);

    let mut controller_drones = HashMap::new();
    let (node_event_send, node_event_recv) = unbounded();

    let mut packet_channels = HashMap::new();
    for drone in config.drone.iter() {
        packet_channels.insert(drone.id, unbounded());
    }
    for client in config.client.iter() {
        packet_channels.insert(client.id, unbounded());
    }
    for server in config.server.iter() {
        packet_channels.insert(server.id, unbounded());
    }

    let mut handles = Vec::new();
    for drone in config.drone.into_iter() {
        // controller
        let (controller_drone_send, controller_drone_recv) = unbounded();
        controller_drones.insert(drone.id, controller_drone_send);
        let node_event_send = node_event_send.clone();
        // packet
        let packet_recv = packet_channels[&drone.id].1.clone();
        let packet_send = drone
            .connected_node_ids
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        handles.push(thread::spawn(move || {
            let mut drone = MyDrone::new(
                drone.id,
                node_event_send,
                controller_drone_recv,
                packet_recv,
                packet_send,
                drone.pdr,
            );

            drone.run();
        }));
    }
    let mut controller = SimulationController::new(
        controller_drones, 
        node_event_recv
    );
    // controller.crash(1, vec![2,3]);

    println!();
    println!();
    println!();
    println!();
    println!("generic_fragment_forward");
    test_fragments::generic_fragment_forward::<MyDrone>();
    println!();
    println!();
    println!();
    println!();
    println!("generic_fragment_drop");
    test_fragments::generic_fragment_drop::<MyDrone>();
    println!();
    println!();
    println!();
    println!();
    println!("generic_chain_fragment_drop");
    test_fragments::generic_chain_fragment_drop::<MyDrone>();
    println!();
    println!();
    println!();
    println!();
    println!("generic_chain_fragment_ack");
    test_fragments::generic_chain_fragment_ack::<MyDrone>();

    
    while let Some(handle) = handles.pop() {
        handle.join().unwrap();
    }

    // ########################################################################################
    // TEST 
    // ########################################################################################

    
    
    // let test_flood = FloodRequest{
    //     flood_id: 99,
    //     initiator_id: 4,
    //     path_trace: vec![],
    // };
    // let p_test_flood = Packet{
    //     routing_header: SourceRoutingHeader{
    //         hop_index: 1,
    //         hops: vec![],
    //     },
    //     session_id: 50,
    //     pack_type: PacketType::FloodRequest(test_flood),
    // };
    // if let Some(c4) = packet_channels.get(&(4 as NodeId)) {
    //     if let Some(d2) = packet_channels.get(&(4 as NodeId)) {
    //         
    //         d2.0.send(p_test_flood).unwrap();
    //     }
    //     if let Some(d3) = packet_channels.get(&(4 as NodeId)) {}
    // }
    
    
    
    
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