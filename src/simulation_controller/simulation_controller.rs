use crate::client_server::network_core::{
    ClientEvent, ClientServerCommand, ServerEvent, ServerType,
};
use crate::simulation_controller::gui::MyApp;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

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
        let crashed_drone_sender = self
            .drones
            .get(&drone_sender_id)
            .map(|(sender, _, _)| sender.clone());

        if let Some((_sender, _, _)) = self.drones.remove(&drone_sender_id) {
            debug!("Removing {} from network...", drone_sender_id);
        }

        for neighbor in neighbors {
            if let Some((neighbor_drone_sender, neighbor_list, _)) = self.drones.get_mut(&neighbor)
            {
                // Remove the crashed drone from the neighbor's list
                neighbor_list.retain(|&id| id != drone_sender_id);

                // Send remove command to the neighbor
                neighbor_drone_sender
                    .send(DroneCommand::RemoveSender(drone_sender_id))
                    .unwrap();
            }
        }

        //Send the crash command after removing the drone
        if let Some(sender) = crashed_drone_sender {
            sender.send(DroneCommand::Crash).unwrap();
        }

        // initialize the first flooding
        self.start_flood_request_for_all();
    }

    /*    pub fn update_topology(&mut self, flood_response: &FloodResponse) {
        // Extract information from flood response
        for &(node_id, node_type) in &flood_response.path_trace {
            match node_type {
                NodeType::Drone => {
                    // Check if this drone is already known
                    if !self.drones.contains_key(&node_id) {
                        // In a real implementation, we would need to obtain these from somewhere
                        // For now, we'll create placeholders
                        let (sender, _) = crossbeam_channel::unbounded();
                        self.drones.insert(node_id, (sender, Vec::new(), 0.0));
                        println!("Added new drone: {}", node_id);
                    }

                    // Update connections for existing drones
                    // This is a simplistic approach - in reality you'd need more sophisticated logic
                    // to determine the full network topology from multiple flood responses
                    for (i, &(curr_id, curr_type)) in flood_response.path_trace.iter().enumerate() {
                        if curr_type == NodeType::Drone {
                            // Look ahead for neighbors
                            if i > 0 {
                                let &(prev_id, prev_type) = &flood_response.path_trace[i-1];
                                if prev_type == NodeType::Drone {
                                    // Add previous node as neighbor if not already present
                                    if let Some((_, neighbors, _)) = self.drones.get_mut(&curr_id) {
                                        if !neighbors.contains(&prev_id) {
                                            neighbors.push(prev_id);
                                        }
                                    }
                                }
                            }

                            if i < flood_response.path_trace.len() - 1 {
                                let &(next_id, next_type) = &flood_response.path_trace[i+1];
                                if next_type == NodeType::Drone {
                                    // Add next node as neighbor if not already present
                                    if let Some((_, neighbors, _)) = self.drones.get_mut(&curr_id) {
                                        if !neighbors.contains(&next_id) {
                                            neighbors.push(next_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                NodeType::Client => {
                    // Check if this client is already known
                    if !self.clients.contains_key(&node_id) {
                        // Create placeholder for new client
                        let (sender, _) = crossbeam_channel::unbounded();

                        // Find neighbors from the path trace
                        let mut neighbors = Vec::new();
                        let client_pos = flood_response.path_trace.iter()
                            .position(|&(id, _)| id == node_id)
                            .unwrap_or(0);

                        if client_pos > 0 {
                            neighbors.push(flood_response.path_trace[client_pos-1].0);
                        }
                        if client_pos < flood_response.path_trace.len() - 1 {
                            neighbors.push(flood_response.path_trace[client_pos+1].0);
                        }

                        self.clients.insert(node_id, (sender, neighbors));
                        println!("Added new client: {}", node_id);
                    }
                },
                // NodeType::Server(server_type) => {
                //     // Check if this server is already known
                //     if !self.servers.contains_key(&node_id) {
                //         // Create placeholder for new server
                //         let (sender, _) = crossbeam_channel::unbounded();
                //
                //         // Find neighbors from the path trace
                //         let mut neighbors = Vec::new();
                //         let server_pos = flood_response.path_trace.iter()
                //             .position(|&(id, _)| id == node_id)
                //             .unwrap_or(0);
                //
                //         if server_pos > 0 {
                //             neighbors.push(flood_response.path_trace[server_pos-1].0);
                //         }
                //         if server_pos < flood_response.path_trace.len() - 1 {
                //             neighbors.push(flood_response.path_trace[server_pos+1].0);
                //         }
                //
                //         // Convert from the flood response server type to our internal type
                //         let server_type_internal = match server_type {
                //             wg_2024::packet::ServerType::Content(content_type) => {
                //                 match content_type {
                //                     wg_2024::packet::ContentType::Text =>
                //                         ServerType::ContentServer(crate::client_server::network_core::ContentType::Text),
                //                     wg_2024::packet::ContentType::Media =>
                //                         ServerType::ContentServer(crate::client_server::network_core::ContentType::Media),
                //                 }
                //             },
                //             wg_2024::packet::ServerType::Communication =>
                //                 ServerType::CommunicationServer,
                //         };
                //
                //         self.servers.insert(node_id, (sender, neighbors, server_type_internal));
                //         println!("Added new server: {}", node_id);
                //     }
                // }
                _ => {}
            }
        }

        // Signal that the topology needs to be redrawn in the GUI
        // This would be handled by the MyApp struct that uses this controller
    }*/

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
    }

    pub fn handle_registration_request(&self, client_id: NodeId, server_id: NodeId) {
        if let Some((client_sender, _)) = self.clients.get(&client_id) {
            client_sender
                .send(ClientServerCommand::RegistrationRequest(server_id))
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
