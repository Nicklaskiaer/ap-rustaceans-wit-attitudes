use crate::client_server::network_core::{
    ClientEvent, ClientServerCommand, ServerEvent, ServerType,
};
use crate::simulation_controller::gui::MyApp;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use crate::message::message::MessageContent::ChatRequest;

pub struct SimulationController {
    drones: HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>, f32)>,
    clients: HashMap<NodeId, (Sender<ClientServerCommand>, Vec<NodeId>)>,
    servers: HashMap<NodeId, (Sender<ClientServerCommand>, Vec<NodeId>, ServerType)>,
    drone_event_recv: Receiver<DroneEvent>,
    client_event_recv: Receiver<ClientEvent>,
    server_event_recv: Receiver<ServerEvent>,
    packet_channels: HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)>,
}

impl SimulationController {
    pub fn new(
        drones: HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>, f32)>,
        clients: HashMap<NodeId, (Sender<ClientServerCommand>, Vec<NodeId>)>,
        servers: HashMap<NodeId, (Sender<ClientServerCommand>, Vec<NodeId>, ServerType)>,
        drone_event_recv: Receiver<DroneEvent>,
        client_event_recv: Receiver<ClientEvent>,
        server_event_recv: Receiver<ServerEvent>,
        packet_channels: HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)>,
    ) -> Self {
        SimulationController {
            drones,
            clients,
            servers,
            drone_event_recv,
            client_event_recv,
            server_event_recv,
            packet_channels,
        }
    }
    
    pub fn handle_remove_sender(&self, drone_sender_id: NodeId, drone_id: NodeId) {
        if let Some((drone_sender, _, _)) = self.drones.get(&drone_sender_id) {
            drone_sender
                .send(DroneCommand::RemoveSender(drone_id))
                .unwrap();
        }
    }

    pub fn handle_add_sender(
        &self,
        drone_sender_id: NodeId,
        drone_id: NodeId,
        drone_packet: Sender<Packet>,
    ) {
        if let Some((drone_sender, _, _)) = self.drones.get(&drone_sender_id) {
            drone_sender
                .send(DroneCommand::AddSender(drone_id, drone_packet))
                .unwrap();
        }
    }

    pub fn handle_set_packet_drop_rate(&mut self, drone_sender_id: NodeId, drop_rate: f32) {
        if let Some((drone_sender, _, stored_rate)) = self.drones.get_mut(&drone_sender_id) {
            *stored_rate = drop_rate;
            drone_sender
                .send(DroneCommand::SetPacketDropRate(drop_rate))
                .unwrap();
        }
    }

    pub fn handle_crash(&mut self, drone_sender_id: NodeId, neighbors: Vec<NodeId>) {
        // Get the drone's data before removing it
        if let Some((drone_sender, neighbors, _)) = self.drones.get(&drone_sender_id).cloned() {
            debug!("Crashing drone {} with {} neighbors...", drone_sender_id, neighbors.len());

            // send RemoveSender commands to all neighbors
            for &neighbor_id in &neighbors {
                if let Some((neighbor_sender, neighbor_list, _)) = self.drones.get_mut(&neighbor_id) {
                    // Update neighbor's list
                    neighbor_list.retain(|&id| id != drone_sender_id);

                    // Send RemoveSender command
                    neighbor_sender.send(DroneCommand::RemoveSender(drone_sender_id)).unwrap();
                }

                if let Some((neighbor_sender, neighbor_list)) = self.clients.get_mut(&neighbor_id) {
                    // Update neighbor's list
                    neighbor_list.retain(|&id| id != drone_sender_id);

                    // Send RemoveSender command
                    neighbor_sender.send(ClientServerCommand::RemoveDrone(drone_sender_id)).unwrap();
                }

                if let Some((neighbor_sender, neighbor_list, _)) = self.servers.get_mut(&neighbor_id) {
                    // Update neighbor's list
                    neighbor_list.retain(|&id| id != drone_sender_id);

                    // Send RemoveSender command
                    neighbor_sender.send(ClientServerCommand::RemoveDrone(drone_sender_id)).unwrap();
                }
            }

            // send the Crash command to the drone and remove it
            drone_sender.send(DroneCommand::Crash).unwrap();
            self.drones.remove(&drone_sender_id);
            
            // initialize another flooding
            self.start_flood_request_for_all();
        }
    }

    // pub fn handle_crash(&mut self, drone_sender_id: NodeId, neighbors: Vec<NodeId>) {
    //     let crashed_drone_sender = self
    //         .drones
    //         .get(&drone_sender_id)
    //         .map(|(sender, _, _)| sender.clone());
    // 
    //     if let Some((_sender, _, _)) = self.drones.remove(&drone_sender_id) {
    //         debug!("Removing {} from network...", drone_sender_id);
    //     }
    // 
    //     for neighbor in neighbors {
    //         if let Some((neighbor_drone_sender, neighbor_list, _)) = self.drones.get_mut(&neighbor)
    //         {
    //             // Remove the crashed drone from the neighbor's list
    //             neighbor_list.retain(|&id| id != drone_sender_id);
    // 
    //             // Send remove command to the neighbor
    //             neighbor_drone_sender
    //                 .send(DroneCommand::RemoveSender(drone_sender_id))
    //                 .unwrap();
    //         }
    //     }
    // 
    //     //Send the crash command after removing the drone
    //     if let Some(sender) = crashed_drone_sender {
    //         sender.send(DroneCommand::Crash).unwrap();
    //     }
    // 
    //     // initialize another flooding
    //     self.start_flood_request_for_all();
    // }

    pub fn get_drone_ids(&self) -> Vec<String> {
        self.drones
            .keys()
            .map(|node_id| format!("Drone {}", node_id.to_string()))
            .collect()
    }

    pub fn get_client_ids(&self) -> Vec<String> {
        self.clients
            .keys()
            .map(|node_id| format!("Client {}", node_id.to_string()))
            .collect()
    }

    pub fn get_server_ids(&self) -> Vec<String> {
        self.servers
            .keys()
            .map(|node_id| format!("Server {}", node_id.to_string()))
            .collect()
    }

    pub fn get_drones(&self) -> &HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>, f32)> {
        &self.drones
    }

    pub fn get_clients(&self) -> &HashMap<NodeId, (Sender<ClientServerCommand>, Vec<NodeId>)> {
        &self.clients
    }

    pub fn get_servers(
        &self,
    ) -> &HashMap<NodeId, (Sender<ClientServerCommand>, Vec<NodeId>, ServerType)> {
        &self.servers
    }

    pub fn get_drone_event_recv(&self) -> &Receiver<DroneEvent> {
        &self.drone_event_recv
    }

    pub fn get_client_event_recv(&self) -> &Receiver<ClientEvent> {
        &self.client_event_recv
    }

    pub fn get_server_event_recv(&self) -> &Receiver<ServerEvent> {
        &self.server_event_recv
    }

    pub fn get_packet_channels(&self) -> &HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)> {
        &self.packet_channels
    }

    pub fn start_flood_request_for_all(&self) {
        for (_, (sender, _)) in &self.clients {
            sender.send(ClientServerCommand::StartFloodRequest).unwrap();
        }

        for (_, (sender, _, _)) in &self.servers {
            sender.send(ClientServerCommand::StartFloodRequest).unwrap();
        }
    }

    pub fn handle_registration_request(&self, client_id: NodeId, server_id: NodeId) {
        if let Some((client_sender, _)) = self.clients.get(&client_id) {
            client_sender
                .send(ClientServerCommand::RegistrationRequest(server_id))
                .unwrap();
        }
    }

    pub fn handle_client_list_request(&self, client_id: NodeId, server_id: NodeId) {
        if let Some((client_sender, _)) = self.clients.get(&client_id) {
            client_sender
                .send(ClientServerCommand::ClientListRequest(server_id))
                .unwrap();
        }
    }

    pub fn handle_send_chat_message(&self, client_id: NodeId, server_id: NodeId, message: String) {
        if let Some((client_sender, _)) = self.clients.get(&client_id) {
            client_sender
                .send(ClientServerCommand::SendChatMessage(server_id, message))
                .unwrap();
        }
    }

    pub fn handle_image_request(&self, client_id: NodeId, server_id: NodeId, image_id: u64) {
        if let Some((client_sender, _)) = self.clients.get(&client_id) {
            client_sender
                .send(ClientServerCommand::RequestImage(server_id, image_id))
                .unwrap();
        }
    }

    pub fn handle_image_list_request(&self, client_id: NodeId, server_id: NodeId) {
        if let Some((client_sender, _)) = self.clients.get(&client_id) {
            client_sender
                .send(ClientServerCommand::RequestImageList(server_id))
                .unwrap();
        }
    }

    pub fn handle_text_list_request(&self, client_id: NodeId, server_id: NodeId) {
        if let Some((client_sender, _)) = self.clients.get(&client_id) {
            client_sender
                .send(ClientServerCommand::RequestTextList(server_id))
                .unwrap();
        }
    }

    pub fn handle_text_request(&self, client_id: NodeId, server_id: NodeId, image_id: u64) {
        if let Some((client_sender, _)) = self.clients.get(&client_id) {
            client_sender
                .send(ClientServerCommand::RequestText(server_id, image_id))
                .unwrap();
        }
    }

    pub fn handle_test_command(&self, node_id: NodeId) {
        if let Some((client_sender, _)) = self.clients.get(&node_id) {
            client_sender
                .send(ClientServerCommand::TestCommand)
                .unwrap();
        }

        if let Some((server_sender, _, _)) = self.servers.get(&node_id) {
            server_sender
                .send(ClientServerCommand::TestCommand)
                .unwrap();
        }
    }
}

pub fn simulation_controller_main(sc: SimulationController) -> Result<(), eframe::Error> {
    // Setup Client and Server
    sc.start_flood_request_for_all();

    #[cfg(feature = "testing")]
    {
        crate::testing::run_tests(&sc);
    }

    // start GUI
    let native_options = eframe::NativeOptions::default();
    let _ctx = egui::Context::default();
    eframe::run_native(
        "Rustaceans Wit Attitudes",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(sc)))),
    )
}
