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
    
    fn try_add_connection(&self, from_id: NodeId, to_id: NodeId) -> bool {
        let mut success = false;
        
        if let Some((sender, _, _)) = self.drones.get(&from_id) {
            if let Some((packet_sender, _)) = self.packet_channels.get(&to_id) {
                match sender.send(DroneCommand::AddSender(to_id, packet_sender.clone())) {
                    Ok(_) => success = true,
                    Err(_) => return false,
                }
            }
        }
        else if let Some((sender, _)) = self.clients.get(&from_id) {
            if let Some((packet_sender, _)) = self.packet_channels.get(&to_id) {
                match sender.send(ClientServerCommand::AddDrone(to_id, packet_sender.clone())) {
                    Ok(_) => success = true,
                    Err(_) => return false,
                }
            }
        }
        else if let Some((sender, _, _)) = self.servers.get(&from_id) {
            if let Some((packet_sender, _)) = self.packet_channels.get(&to_id) {
                match sender.send(ClientServerCommand::AddDrone(to_id, packet_sender.clone())) {
                    Ok(_) => success = true,
                    Err(_) => return false,
                }
            }
        }

        success
    }
    fn try_remove_connection(&mut self, from_id: NodeId, to_id: NodeId) -> bool {
        let mut success = false;
        
        if let Some((sender, neighbors, _)) = self.drones.get_mut(&from_id) {
            match sender.send(DroneCommand::RemoveSender(to_id)) {
                Ok(_) => {
                    neighbors.retain(|&id| id != to_id);
                    success = true;
                }
                Err(_) => return false,
            }
        }
        else if let Some((sender, neighbors)) = self.clients.get_mut(&from_id) {
            match sender.send(ClientServerCommand::RemoveDrone(to_id)) {
                Ok(_) => {
                    neighbors.retain(|&id| id != to_id);
                    success = true;
                }
                Err(_) => return false,
            }
        }
        else if let Some((sender, neighbors, _)) = self.servers.get_mut(&from_id) {
            match sender.send(ClientServerCommand::RemoveDrone(to_id)) {
                Ok(_) => {
                    neighbors.retain(|&id| id != to_id);
                    success = true;
                }
                Err(_) => return false,
            }
        }

        success
    }
    fn update_neighbor_list(&mut self, node_id: NodeId, neighbor_id: NodeId, should_add: bool) {
        // Update drone neighbors
        if let Some((_, neighbors, _)) = self.drones.get_mut(&node_id) {
            if should_add {
                if !neighbors.contains(&neighbor_id) {
                    neighbors.push(neighbor_id);
                }
            } else {
                neighbors.retain(|&id| id != neighbor_id);
            }
        }
        // Update client neighbors
        else if let Some((_, neighbors)) = self.clients.get_mut(&node_id) {
            if should_add {
                if !neighbors.contains(&neighbor_id) {
                    neighbors.push(neighbor_id);
                }
            } else {
                neighbors.retain(|&id| id != neighbor_id);
            }
        }
        // Update server neighbors
        else if let Some((_, neighbors, _)) = self.servers.get_mut(&node_id) {
            if should_add {
                if !neighbors.contains(&neighbor_id) {
                    neighbors.push(neighbor_id);
                }
            } else {
                neighbors.retain(|&id| id != neighbor_id);
            }
        }
    }
    
    pub fn handle_add_sender(&mut self, node1_id: NodeId, node2_id: NodeId) -> bool {
        let node1_added_node2 = self.try_add_connection(node1_id, node2_id);
        debug!("did {} added {}? {}", node1_id, node2_id, node1_added_node2);
        let node2_added_node1 = self.try_add_connection(node2_id, node1_id);
        debug!("did {} added {}? {}", node2_id, node1_id, node2_added_node1);
        
        
        if node1_added_node2 && node2_added_node1 {
            self.update_neighbor_list(node1_id, node2_id, true);
            self.update_neighbor_list(node2_id, node1_id, true);

            self.start_flood_request_for_all();
            return true;
        }
        false
    }
    
    pub fn handle_remove_sender(&mut self, node1_id: NodeId, node2_id: NodeId) -> bool {
        let node1_removed_node2 = self.try_remove_connection(node1_id, node2_id);
        debug!("did {} removed {}? {}", node1_id, node2_id, node1_removed_node2);
        let node2_removed_node1 = self.try_remove_connection(node2_id, node1_id);
        debug!("did {} removed {}? {}", node2_id, node1_id, node2_removed_node1);

        if node1_removed_node2 && node2_removed_node1 {
            self.update_neighbor_list(node1_id, node2_id, false);
            self.update_neighbor_list(node2_id, node1_id, false);
            
            self.start_flood_request_for_all();
            return true;
        }
        false
    }

    pub fn handle_set_packet_drop_rate(&mut self, drone_sender_id: NodeId, drop_rate: f32) {
        if let Some((drone_sender, _, stored_rate)) = self.drones.get_mut(&drone_sender_id) {
            *stored_rate = drop_rate;
            drone_sender.send(DroneCommand::SetPacketDropRate(drop_rate)).unwrap();
        }
    }

    pub fn handle_crash(&mut self, drone_sender_id: NodeId) {
        // Get the drone's data before removing it
        if let Some((drone_sender, neighbors, _)) = self.drones.get(&drone_sender_id).cloned() {
            debug!("Crashing drone {} with {} neighbors...", drone_sender_id, neighbors.len());

            // Remove connections from all neighbors to the crashing drone
            for &neighbor_id in &neighbors {
                self.try_remove_connection(neighbor_id, drone_sender_id);
            }

            // Send the Crash command to the drone and remove it
            drone_sender.send(DroneCommand::Crash).unwrap();
            self.drones.remove(&drone_sender_id);

            // Initialize another flooding
            self.start_flood_request_for_all();
        }
    }

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
    
    pub fn handle_print_all_node_data_command(&self, node_id: NodeId) {
        if let Some((client_sender, _)) = self.clients.get(&node_id) {
            client_sender
                .send(ClientServerCommand::PrintAllNodeData)
                .unwrap();
        }

        if let Some((server_sender, _, _)) = self.servers.get(&node_id) {
            server_sender
                .send(ClientServerCommand::PrintAllNodeData)
                .unwrap();
        }
    }
}

pub fn simulation_controller_main(sc: SimulationController) -> Result<(), eframe::Error> {
    // Setup Client and Server
    sc.start_flood_request_for_all();

    // start GUI
    let native_options = eframe::NativeOptions::default();
    let _ctx = egui::Context::default();
    eframe::run_native(
        "Rustaceans Wit Attitudes",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(sc)))),
    )
}
