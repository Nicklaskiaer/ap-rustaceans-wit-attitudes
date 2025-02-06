use std::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use crate::client::client::ClientEvent;
use crate::server::server::ServerEvent;
use crate::simulation_controller::gui::MyApp;

pub struct SimulationController {
    drones: HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)>,
    clients: HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)>, //TODO: create a ClientCommand
    servers: HashMap<NodeId, Vec<NodeId>>,
    drone_event_recv: Receiver<DroneEvent>,
    client_event_recv: Receiver<ClientEvent>,
    server_event_recv: Receiver<ServerEvent>,
}

impl SimulationController {
    pub fn new(
        drones: HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)>,
        clients: HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)>,
        servers: HashMap<NodeId, Vec<NodeId>>,
        drone_event_recv: Receiver<DroneEvent>,
        client_event_recv: Receiver<ClientEvent>,
        server_event_recv: Receiver<ServerEvent>,
    ) -> Self {
        SimulationController {
            drones,
            clients,
            servers,
            drone_event_recv,
            client_event_recv,
            server_event_recv,
        }
    }

    pub fn handle_remove_sender(&self, drone_sender_id: NodeId, drone_id: NodeId) {
        if let Some((drone_sender, _)) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::RemoveSender(drone_id)).unwrap();
        }
    }

    pub fn handle_add_sender(&self, drone_sender_id: NodeId, drone_id: NodeId, drone_packet: Sender<Packet>) {
        if let Some((drone_sender, _)) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::AddSender(drone_id, drone_packet)).unwrap();
        }
    }

    pub fn handle_set_packet_drop_rate(&self, drone_sender_id: NodeId, drop_rate: f32) {
        if let Some((drone_sender, _)) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::SetPacketDropRate(drop_rate)).unwrap();
        }
    }

    pub fn handle_crash(&mut self, drone_sender_id: NodeId, neighbors: Vec<NodeId>) {
        if let Some((crashed_drone_sender, _)) = self.drones.get(&drone_sender_id) {
            // send crash command to the drone
            crashed_drone_sender.send(DroneCommand::Crash).unwrap();

            for neighbor in neighbors {
                if let Some((neighbor_drone_sender, _)) = self.drones.get(&neighbor) {
                    // remove drone from neighbor
                    neighbor_drone_sender.send(DroneCommand::RemoveSender(drone_sender_id)).unwrap();

                    // remove neighbor form drone
                    crashed_drone_sender.send(DroneCommand::RemoveSender(neighbor)).unwrap();
                }
            }

        }
    }

    pub fn get_drone_ids(&self) -> Vec<String> {
        self.drones.keys()
            .map(|node_id| format!("Drone {}", node_id.to_string()))
            .collect()
    }

    pub fn get_client_ids(&self) -> Vec<String> {
        self.clients.keys()
            .map(|node_id| format!("Drone {}", node_id.to_string()))
            .collect()
    }

    pub fn get_server_ids(&self) -> Vec<String> {
        self.servers.keys()
            .map(|node_id| format!("Drone {}", node_id.to_string()))
            .collect()
    }

    pub fn get_drones(&self) -> HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)> {
        self.drones.clone()
    }

    pub fn get_clients(&self) -> HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)> {
        self.clients.clone()
    }

    pub fn get_servers(&self) -> HashMap<NodeId, Vec<NodeId>> {
        self.servers.clone()
    }

    pub fn get_drone_event_recv(&self) -> Receiver<DroneEvent>{
        self.drone_event_recv.clone()
    }

    pub fn get_client_event_recv(&self) -> Receiver<ClientEvent>{
        self.client_event_recv.clone()
    }

    pub fn get_server_event_recv(&self) -> Receiver<ServerEvent>{
        self.server_event_recv.clone()
    }
}


pub fn simulation_controller_main(sc: SimulationController) -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rustaceans Wit Attitudes",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(sc))))
    )
}