#[cfg(feature = "debug")]
use crate::debug;

use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use std::thread;
use std::thread::ThreadId;
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet;
use wg_2024::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType,
};
use rand::{Rng, thread_rng, random};
use crate::assembler::assembler::Assembler;
use crate::client::client_server_command::ClientServerCommand;
use crate::server::message::{Message, TextRequest};
use crate::server::server::ServerEvent;

pub struct Client {
    id: NodeId,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ClientEvent>,
    controller_recv: Receiver<ClientServerCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    assemblers: Vec<Assembler>,
    assembler_send: Sender<Vec<u8>>,
    assembler_recv: Receiver<Vec<u8>>,
}

pub enum ClientEvent {
    PacketSent(Packet),
    PacketReceived(Packet),
}

pub trait ClientTrait {
    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientServerCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        assemblers: Vec<Assembler>,
        topology_map: HashSet<(NodeId, Vec<NodeId>)>,
        assembler_send: Sender<Vec<u8>>,
        assembler_recv: Receiver<Vec<u8>>,
    ) -> Self;

    fn run(&mut self);

    // fn send_fragment_to_assembler(&mut self, packet: Packet) -> Result<String, String>;
    //
    // fn handle_flood_response(
    //     &mut self,
    //     sender_node_id: NodeId,
    //     flood_response: FloodResponse,
    // ) -> Result<String, String>;
    //
    // fn send_response(&mut self, message: Message<TextRequest>)
    //                  -> Result<Packet, SendError<Packet>>;
    //
    // fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ClientEvent>>;
    // fn send_recv_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ClientEvent>>;

    // fn compose_message(
    //     source_id: NodeId,
    //     session_id: u64,
    //     raw_content: String,
    // ) -> Result<Message<Self::RequestType>, String> {
    //     let content = Self::RequestType::from_string(raw_content)?;
    //     Ok(Message {
    //         session_id,
    //         source_id,
    //         content,
    //     })
    // }
    // fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String>;
}

impl ClientTrait for Client {
    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientServerCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        assemblers: Vec<Assembler>,
        topology_map: HashSet<(NodeId, Vec<NodeId>)>,
        assembler_send: Sender<Vec<u8>>,
        assembler_recv: Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            id,
            connected_drone_ids,
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
            assemblers,
            topology_map,
            assembler_send,
            assembler_recv,
        }
    }

    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.handle_command(command);
                    }
                },
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.handle_packet(packet);
                    }
                },
                recv(self.assembler_recv) -> data => {
                    if let Ok(data) = data {
                        // TODO: handle assembled data
                    }
                },
            }
        }
    }
}

impl Client {
    fn handle_command(&mut self, command: ClientServerCommand) {
        match command {
            ClientServerCommand::DroneCmd(drone_cmd) => {
                // Handle drone command
                match drone_cmd {
                    DroneCommand::SetPacketDropRate(_) => {},
                    DroneCommand::Crash => {},
                    DroneCommand::AddSender(id, sender) => {},
                    DroneCommand::RemoveSender(id) => {},
                }
            },
            // ClientServerCommand::RegistrationRequest(node_id) => {
            //     // Handle registration request
            // },
            // ClientServerCommand::RequestServerList(node_id) => {
            //     // Handle server list request
            // },
            // ClientServerCommand::RequestFileList(node_id) => {
            //     // Handle file list request
            // },
            ClientServerCommand::SendChatMessage(node_id, id, msg) => {
                debug!("Client: {:?} sending chat message to {:?}: {}", self.id, node_id, msg);

                // Create a session ID using the current timestamp and message ID
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let session_id = timestamp ^ (id as u64);

                // Create a TextRequest message
                // Note: In a real implementation, we would serialize the chat message
                // properly, but for this example we'll use TextRequest
                let message = Message {
                    source_id: self.id,
                    session_id,
                    content: TextRequest::Text(id as u64), // Ideally we'd encode the message
                };

                // Compute path to the destination node
                match self.compute_path_to_node(node_id) {
                    Ok(path) => {
                        debug!("Client: {:?} found path to {:?}: {:?}", self.id, node_id, path);

                        // Send the message
                        match self.send_response(message) {
                            Ok(packet) => {
                                debug!("Client: {:?} sent chat message to {:?}", self.id, node_id);
                            },
                            Err(e) => {
                                debug!("ERROR: Client: {:?} failed to send chat message to {:?}: {:?}", 
                                  self.id, node_id, e);
                            }
                        };
                    },
                    Err(e) => {
                        debug!("ERROR: Client: {:?} could not compute path to {:?}: {}", 
                          self.id, node_id, e);
                    }
                }
            },

            ClientServerCommand::StartFloodRequest => {
                debug!("Client: {:?} received StartFloodRequest command", self.id);

                // Generate a unique flood ID using current time
                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                let flood_id = timestamp ^ random::<u64>();
                
                // Create path trace with just this client
                let path_trace = vec![(self.id, NodeType::Client)];

                // Send flood request to all connected drones
                for drone_id in &self.connected_drone_ids {
                    if let Some(sender) = self.packet_send.get(drone_id) {
                        let flood_request = Packet::new_flood_request(
                            SourceRoutingHeader {
                                hop_index: 1,
                                hops: vec![self.id, *drone_id],
                            },
                            flood_id,
                            FloodRequest {
                                flood_id,
                                initiator_id: self.id,
                                path_trace: path_trace.clone(),
                            },
                        );

                        // Send the packet and notify the sc
                        match sender.send(flood_request.clone()) {
                            Ok(_) => {
                                // TODO: send to GUI
                                debug!("Client: {:?} sent flood request to {:?}", self.id, drone_id);
                            },
                            Err(e) => {
                                debug!("ERROR, Client: {:?} was not able to send flood request to {:?}", self.id, drone_id);
                            }
                        }
                    }
                }
            },
        }
    }
    fn handle_packet(&mut self, mut packet: Packet) {
        match &packet.pack_type {
            PacketType::MsgFragment(_fragment) => {
                debug!("Client: {:?} received a MsgFragment {:?}", self.id, _fragment);
                
                // handle message fragment
                let message_fragment_result = self.send_fragment_to_assembler(packet);
                match message_fragment_result {
                    Ok(_) => {},
                    Err(e) => {
                        // println!("Error: {}", e);
                    }
                }
            },
            PacketType::FloodResponse(_flood_response) => {
                debug!("Client: {:?} received a FloodResponse {:?}", self.id, _flood_response);

                // Extract path from flood response and add current node
                let mut new_path: Vec<u8> = _flood_response.path_trace.iter().map(|(id, _)| *id).collect();
                new_path.push(self.id);

                // Get sender node ID from packet routing header
                let sender_node_id = packet.routing_header.hops[packet.routing_header.hop_index];

                // Update topology map based on new path information
                let update_result = if !self.topology_map.contains(&(sender_node_id, new_path.clone())) {
                    // Case 1: New node entry
                    self.topology_map.insert((sender_node_id, new_path));
                    "new node added to topology map"
                } else {
                    // Case 2: Existing node - check if new path has better information
                    if let Some((_, existing_path)) = self.topology_map.iter().find(|(id, _)| *id == sender_node_id) {
                        if _flood_response.path_trace.len() > existing_path.len() {
                            // Replace with better path
                            self.topology_map.remove(&(sender_node_id, existing_path.clone()));
                            self.topology_map.insert((sender_node_id, new_path));
                            "node found in topology map but updated with better path"
                        } else {
                            "node already in topology map with equal or better path"
                        }
                    } else {
                        "inconsistent topology map state"
                    }
                };

                debug!("Client: {:?} - {}: {:?}", self.id, update_result, self.topology_map);

                //TODO: after receiving all flood request send request to all servers to get their type
            },
            PacketType::Ack(_ack) => {
                debug!("Client: {:?} received a Ack {:?}", self.id, _ack);
                // handle ack
            },
            PacketType::Nack(_) => {
                // send received packet to simulation controller
                // let sc_send_res = self.send_recv_to_sc(packet.clone());
                // match sc_send_res {
                //     Ok(_) => {},
                //     Err(e) => {
                //         println!("Error: {}", e);
                //     }
                // }
            },
            PacketType::FloodRequest(_flood_request) => {
                debug!("Client: {:?} received a FloodRequest {:?}", self.id, _flood_request);
                // handle flood request
            },
        }
    }





    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ClientEvent>> {
        self.controller_send.send(ClientEvent::PacketSent(packet))
    }
    fn send_recv_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ClientEvent>> {
            self.controller_send.send(ClientEvent::PacketReceived(packet))
    }
    fn send_fragment_to_assembler(&mut self, packet: Packet) -> Result<String, String> {
        // Send the data and the fragment index to the assembler
        for assembler in self.assemblers.iter_mut() {
            if assembler.session_id == packet.session_id {
                assembler.packet_send.send(packet).unwrap();
                return Ok("Sent fragment to assembler".to_string());
            }
        }

        // If the assembler does not exist, create a new one
        let (packet_send, packet_recv) = unbounded();
        let (server_send, server_recv) = unbounded();
        let assembler = Assembler::new(
            packet.session_id,
            packet_send,
            packet_recv,
            server_send,
            server_recv,
        );

        // Send the data and the fragment index to the assembler
        match assembler.packet_send.send(packet) {
            Ok(_) => {}
            Err(_) => {
                return Err("Failed to send packet to assembler".to_string());
            }
        }

        // Add new assembler to the list
        self.assemblers.push(assembler);

        return Ok("Sent fragment to assembler".to_string());
    }
    fn send_response(&mut self, message: Message<TextRequest>) -> Result<Packet, SendError<Packet>> {
        // compute the hops
        let mut hops = Vec::new();
        if let Ok(computed_hops) = self.compute_path_to_node(message.source_id) {
            hops = computed_hops;
        }

        // create source header
        let source_routing_header = SourceRoutingHeader::new(hops, 1);

        let packet = Packet::new_fragment(
            source_routing_header,
            message.session_id,
            Fragment::new(0, 1, [0; 128]), // example data
        );
        debug!("assssssssssssssssss {:?}", packet);

        // send packet
        if let Some(sender) = self.packet_send.get(&message.source_id) {
            // send packet
            match sender.send(packet.clone()) {
                Ok(_) => {
                    // send "sent packet" to simulation controller
                    let sc_send_res = self.send_sent_to_sc(packet.clone());
                    match sc_send_res {
                        Ok(_) => {}
                        Err(e) => {
                            // println!("Error: {}", e);
                        }
                    }
                    Ok(packet)
                }
                Err(e) => Err(e),
            }
        } else {
            Err(SendError(packet))
        }
    }
    // find the route to the node in the hashmap, and return the path
    // fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String> {
    //     debug!("compute_path_to_node, target_node_id: {:?}", target_node_id);
    //     debug!("compute_path_to_node, self.topology_map): {:?}", self.topology_map);
    //     
    //     let path = self
    //         .topology_map
    //         .iter()
    //         .find(|(id, _)| *id == target_node_id);
    // 
    //     match path {
    //         Some((_, path)) => Ok(path.clone()),
    //         None => Err("Path not found".to_string()),
    //     }
    // }

    fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String> {
        debug!("compute_path_to_node, target_node_id: {:?}", target_node_id);
        debug!("compute_path_to_node, self.topology_map): {:?}", self.topology_map);

        // Find paths that contain the target node
        for (_, path) in &self.topology_map {
            // Check if the target node is in this path
            if let Some(pos) = path.iter().position(|&id| id == target_node_id) {
                // Found the target node in this path
                // Extract the subpath from the beginning up to and including the target node
                return Ok(path[0..=pos].to_vec());
            }
        }

        // No path found containing the target node
        Err("Path not found".to_string())
    }
}