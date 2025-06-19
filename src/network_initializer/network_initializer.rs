#[cfg(feature = "debug")]
use crate::debug;

use crate::client_server::client::Client;
use crate::client_server::communication_server::CommunicationServer;
use crate::client_server::content_server::ContentServer;
use crate::client_server::network_core::{
    ClientEvent, ClientServerCommand, ContentType, NetworkNode, ServerEvent, ServerType,
};
use crate::simulation_controller::simulation_controller::{
    simulation_controller_main, SimulationController,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rustaceans_wit_attitudes::RustaceansWitAttitudesDrone;
use std::collections::HashMap;
use std::collections::HashSet;
use std::{fs, thread};
use skylink::SkyLinkDrone;
use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

const NUM_CONTENT_SERVERS: usize = 4;

pub fn main() {
    // let current_path = env::current_dir().expect("Unable to get current directory");
    // println!("Current path: {:?}", current_path);
    let config;
    #[cfg(feature = "testing")]
    {
        config = parse_config("src/test_config.toml");
    }

    #[cfg(not(feature = "testing"))]
    {
        config = parse_config("src/config.toml");
    }

    // check for errors in the toml
    check_toml_validity(&config);

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
    let (node_event_send_drone, node_event_recv_drone): (Sender<DroneEvent>, Receiver<DroneEvent>) =
        unbounded();
    let mut controller_drones = HashMap::new();
    for drone in config.drone.into_iter() {
        // controller
        let (controller_drone_send, controller_drone_recv): (
            Sender<DroneCommand>,
            Receiver<DroneCommand>,
        ) = unbounded();
        controller_drones.insert(
            drone.id,
            (
                controller_drone_send,
                drone.connected_node_ids.clone(),
                drone.pdr,
            ),
        );
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
            // let mut drone = SkyLinkDrone::new(
            //     drone.id,
            //     node_event_send_drone,
            //     controller_drone_recv,
            //     packet_recv,
            //     packet_send,
            //     drone.pdr,
            // );
            
            let mut drone = RustaceansWitAttitudesDrone::new(
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
    let (node_event_send_client, node_event_recv_client): (
        Sender<ClientEvent>,
        Receiver<ClientEvent>,
    ) = unbounded();
    let mut controller_clients = HashMap::new();
    for client in config.client.into_iter() {
        // controller
        let (controller_client_send, controller_client_recv): (
            Sender<ClientServerCommand>,
            Receiver<ClientServerCommand>,
        ) = unbounded();
        controller_clients.insert(
            client.id,
            (
                controller_client_send.clone(),
                client.connected_drone_ids.clone(),
            ),
        );
        let node_event_send_client = node_event_send_client.clone();

        // packet
        let packet_recv: Receiver<Packet> = packet_channels[&client.id].1.clone();
        let packet_send: HashMap<NodeId, Sender<Packet>> = client
            .connected_drone_ids
            .clone()
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        // spawn
        let (assembler_send, assembler_recv) = unbounded();
        let (assembler_send_res, assembler_recv_res) = unbounded();
        thread::spawn(move || {
            let mut client = Client::new(
                client.id,
                client.connected_drone_ids,
                node_event_send_client,
                controller_client_send,
                controller_client_recv,
                packet_send,
                packet_recv,
                vec![],
                HashSet::new(),
                HashMap::new(),
                HashSet::new(),
                assembler_send,
                assembler_recv,
                assembler_send_res,
                assembler_recv_res,
            );

            client.run();
        });
    }

    // INITIALIZE SERVERS
    // Ensure that there are enough servers
    if config.server.len() < NUM_CONTENT_SERVERS {
        panic!(
            "Not enough servers to allocate {} content servers",
            NUM_CONTENT_SERVERS
        );
    }
    // Load file IDs from directories
    let text_files: Vec<u64> = match fs::read_dir("server_content/text_files") {
        Ok(entries) => entries
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let file_name = path.file_stem()?.to_str()?;
                file_name.parse::<u64>().ok()
            })
            .collect(),
        Err(_) => vec![],
    };
    let media_files: Vec<u64> = match fs::read_dir("server_content/media_files") {
        Ok(entries) => entries
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let file_name = path.file_stem()?.to_str()?;
                file_name.parse::<u64>().ok()
            })
            .collect(),
        Err(_) => vec![],
    };
    debug!(
        "Found {} text files and {} media files",
        text_files.len(),
        media_files.len()
    );
    // Split files for each content server
    let mut available_text_files = text_files.clone();
    let mut available_media_files = media_files.clone();
    available_text_files.shuffle(&mut thread_rng());
    available_media_files.shuffle(&mut thread_rng());
    // Create server type assignments with content distribution
    let server_types_with_content = {
        let mut types = Vec::with_capacity(config.server.len());

        // Create content servers (half text, half media)
        let text_server_count = NUM_CONTENT_SERVERS / 2 + NUM_CONTENT_SERVERS % 2;
        let media_server_count = NUM_CONTENT_SERVERS - text_server_count;

        // Text content servers
        for i in 0..text_server_count {
            let start_idx = i * available_text_files.len() / text_server_count;
            let end_idx = (i + 1) * available_text_files.len() / text_server_count;
            let server_files = available_text_files[start_idx..end_idx].to_vec();
            types.push((ServerType::ContentServer(ContentType::Text), server_files));
        }

        // Media content servers
        for i in 0..media_server_count {
            let start_idx = i * available_media_files.len() / media_server_count;
            let end_idx = (i + 1) * available_media_files.len() / media_server_count;
            let server_files = available_media_files[start_idx..end_idx].to_vec();
            types.push((ServerType::ContentServer(ContentType::Media), server_files));
        }

        // Remaining servers are communication servers
        types.extend(vec![
            (ServerType::CommunicationServer, vec![]);
            config.server.len() - NUM_CONTENT_SERVERS
        ]);
        types.shuffle(&mut thread_rng());
        types
    };
    let (node_event_send_server, node_event_recv_server): (
        Sender<ServerEvent>,
        Receiver<ServerEvent>,
    ) = unbounded();
    let mut controller_servers = HashMap::new();
    for (server, (server_type, files)) in config
        .server
        .into_iter()
        .zip(server_types_with_content.into_iter())
    {
        // controller
        let (controller_server_send, controller_server_recv): (
            Sender<ClientServerCommand>,
            Receiver<ClientServerCommand>,
        ) = unbounded();
        controller_servers.insert(
            server.id,
            (
                controller_server_send,
                server.connected_drone_ids.clone(),
                server_type.clone(),
            ),
        );
        let node_event_send_server = node_event_send_server.clone();

        // packet
        let packet_recv: Receiver<Packet> = packet_channels[&server.id].1.clone();
        let packet_send: HashMap<NodeId, Sender<Packet>> = server
            .connected_drone_ids
            .clone()
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        // spawn
        let (assembler_send, assembler_recv) = unbounded();
        let (assembler_send_res, assembler_recv_res) = unbounded();
        match server_type {
            ServerType::ContentServer(content_type) => {
                debug!(
                    "Creating ContentServer id={} with content_type={:?} and {} files",
                    server.id,
                    content_type,
                    files.len()
                );

                let content_type_clone = content_type.clone();
                let files_clone = files.clone();

                thread::spawn(move || {
                    let mut server = ContentServer::new(
                        server.id,
                        server.connected_drone_ids,
                        node_event_send_server,
                        controller_server_recv,
                        packet_send,
                        packet_recv.clone(),
                        vec![],
                        HashSet::new(),
                        assembler_send,
                        assembler_recv,
                        assembler_send_res,
                        assembler_recv_res,
                        content_type_clone,
                        files_clone,
                    );
                    server.run();
                });
            }
            ServerType::CommunicationServer => {
                thread::spawn(move || {
                    let mut server = CommunicationServer::new(
                        server.id,
                        server.connected_drone_ids,
                        node_event_send_server,
                        controller_server_recv,
                        packet_send,
                        packet_recv.clone(),
                        vec![],
                        HashSet::new(),
                        assembler_send,
                        assembler_recv,
                        assembler_send_res,
                        assembler_recv_res,
                        HashSet::new(),
                        Vec::new(),
                    );
                    server.run();
                });
            }
        }
    }

    // INITIALIZE SIMULATION CONTROLLER AND GUI
    // THE SC WILL ALSO START THE FIRST FLOOD REQUEST
    let sc = SimulationController::new(
        controller_drones,
        controller_clients,
        controller_servers,
        node_event_recv_drone,
        node_event_recv_client,
        node_event_recv_server,
        packet_channels,
    );

    simulation_controller_main(sc).expect("GUI panicked!");
}

fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).unwrap();
    toml::from_str(&file_str).unwrap()
}

fn check_toml_validity(config: &Config) {
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
            panic!(
                "found repetition of id across drones, clients, and servers: {}!",
                drone.id
            );
        }
    }

    for client in &config.client {
        if !client_ids.insert(client.id) {
            panic!("found repetition of id: {}!", client.id)
        }
        if !all_ids.insert(client.id) {
            panic!(
                "found repetition of id across drones, clients, and servers: {}!",
                client.id
            );
        }
    }

    for server in &config.server {
        if !server_ids.insert(server.id) {
            panic!("found repetition of id: {}!", server.id)
        }
        if !all_ids.insert(server.id) {
            panic!(
                "found repetition of id across drones, clients, and servers: {}!",
                server.id
            );
        }
    }
    // </editor-fold>

    // <editor-fold desc="Drone, PDR and connected_node_ids">
    let min_pdr = 0.00;
    let max_pdr = 1.00;
    for drone in &config.drone {
        // do drones have all unique connected_node_ids without their id and with no repetition?
        let mut c_drones_ids = HashSet::new();
        for connected_drone in &drone.connected_node_ids {
            if *connected_drone == drone.id {
                panic!("the drone {} has its id in connected_node_ids!", drone.id)
            }
            if !c_drones_ids.insert(connected_drone) {
                panic!(
                    "the drone {} has id repetition in connected_node_ids!",
                    drone.id
                )
            }
        }

        // do drones have a pdr between 0.05% and 5%?
        if drone.pdr < min_pdr || drone.pdr > max_pdr {
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
        for connected_drone in &client.connected_drone_ids {
            if !drone_ids.contains(connected_drone) {
                panic!(
                    "the client {} has an invalid id in connected_node_ids: {}!",
                    client.id, connected_drone
                );
            }
            if !c_client_ids.insert(connected_drone) {
                panic!(
                    "the client {} has id repetition in connected_node_ids!",
                    client.id
                )
            }
        }

        // do client the right numbers of drones?
        let n_drones = client.connected_drone_ids.iter().len();
        if n_drones < min_drones || n_drones > max_drones {
            panic!("{} has an invalid number of drones", client.id)
        }
    }
    // </editor-fold>

    // <editor-fold desc="Server, n. drones connected and connection id repetition">
    let min_drones = 2;
    for server in &config.server {
        // do servers have all unique connected_node_ids with no repetition and without any clients or servers id?
        let mut c_server_ids = HashSet::new();
        for connected_drone in &server.connected_drone_ids {
            if !drone_ids.contains(connected_drone) {
                panic!(
                    "the server {} has an invalid id in connected_node_ids: {}!",
                    server.id, connected_drone
                )
            }
            if !c_server_ids.insert(connected_drone) {
                panic!(
                    "the server {} has id repetition in connected_node_ids!",
                    server.id
                )
            }
        }

        // do client the right numbers of drones?
        let n_drones = server.connected_drone_ids.iter().len();
        if !(n_drones >= min_drones) {
            panic!("{} has an invalid number of drones", server.id)
        }
    }
    // </editor-fold>

    // <editor-fold desc="check for bidirectional connectivity">
    // Check bidirectional connectivity for all nodes
    let mut connection_map: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();

    // Build connection map for all nodes
    for drone in &config.drone {
        connection_map
            .entry(drone.id)
            .or_insert_with(HashSet::new)
            .extend(&drone.connected_node_ids);
    }

    for client in &config.client {
        connection_map
            .entry(client.id)
            .or_insert_with(HashSet::new)
            .extend(&client.connected_drone_ids);
    }

    for server in &config.server {
        connection_map
            .entry(server.id)
            .or_insert_with(HashSet::new)
            .extend(&server.connected_drone_ids);
    }

    // Check that all connections are bidirectional
    for (node_id, connections) in &connection_map {
        for &connected_id in connections {
            if !connection_map
                .get(&connected_id)
                .map_or(false, |conns| conns.contains(node_id))
            {
                panic!(
                    "Unidirectional connection: Node {} is connected to Node {}, but Node {} is not connected back to Node {}",
                    node_id, connected_id, connected_id, node_id
                );
            }
        }
    }
    // </editor-fold>
}
