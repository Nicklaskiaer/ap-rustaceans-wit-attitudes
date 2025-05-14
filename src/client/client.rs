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
use crate::server::server::{ServerEvent, ServerType};

pub struct Client {
    id: NodeId,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    server_type_map: HashMap<NodeId, ServerType>,
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
        server_type_map: HashMap<NodeId, ServerType>,
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
        server_type_map: HashMap<NodeId, ServerType>,
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
            server_type_map,
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
            // ClientServerCommand::SendChatMessage(node_id, id, msg) => {
            //     debug!("Client: {:?} sending chat message to {:?}: {}", self.id, node_id, msg);
            //
            //     // Create a session ID
            //     // let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
            //     // let session_id = timestamp ^ random::<u64>();
            //     let session_id = id as u64;
            //
            //     // Compute path to the destination node
            //     match self.compute_path_to_node(node_id) {
            //         Ok(path) => {
            //             debug!("Client: {:?} found path to {:?}: {:?}", self.id, node_id, path);
            //
            //             // create packet
            //             let target_node_id = 1;
            //             let source_routing_header = SourceRoutingHeader::new(path, target_node_id);
            //             let packet = Packet::new_fragment(
            //                 source_routing_header,
            //                 session_id,
            //                 Fragment::new(0, 1, [0; 128]), // example data,
            //             );
            //
            //             // Send the message
            //             if let Some(sender) = self.packet_send.get(&node_id) {
            //                 match sender.send(packet) {
            //                     Ok(_) => {
            //                         debug!("Client: {:?} sent chat message to {:?}", self.id, node_id);
            //                     },
            //                     Err(e) => {
            //                         debug!("ERROR: Client: {:?} failed to send chat message to {:?}: {:?}", self.id, node_id, e);
            //                     }
            //                 }
            //             }
            //         },
            //         Err(e) => {
            //             debug!("ERROR: Client: {:?} could not compute path to {:?}: {}", self.id, node_id, e);
            //         }
            //     }
            // },
            ClientServerCommand::SendChatMessage(node_id, id, msg) => {
                debug!("Client: {:?} sending chat message to {:?}: {}", self.id, node_id, msg);

                // Create a session ID
                // let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                // let session_id = timestamp ^ random::<u64>();
                let session_id = id as u64;

                // Convert message to bytes
                let msg_bytes = msg.into_bytes();

                // Calculate how many fragments needed
                let total_fragments = (msg_bytes.len() + 127) / 128;

                // Compute path to the destination node
                match self.compute_path_to_node(node_id) {
                    Ok(path) => {
                        debug!("Client: {:?} found path to {:?}: {:?}", self.id, node_id, path);

                        // create source_routing_header
                        let target_node_id = 1;
                        let source_routing_header = SourceRoutingHeader::new(path.clone(), target_node_id);

                        // Split message into fragments and send
                        for i in 0..total_fragments {
                            let start = i * 128;
                            let end = std::cmp::min((i + 1) * 128, msg_bytes.len());
                            let chunk_size = end - start;

                            // Create data array with 128 bytes, fill with message data
                            let mut data = [0u8; 128];
                            data[..chunk_size].copy_from_slice(&msg_bytes[start..end]);

                            // Create fragment
                            let fragment = Fragment {
                                fragment_index: i as u64,
                                total_n_fragments: total_fragments as u64,
                                length: chunk_size as u8,
                                data,
                            };

                            // Create packet
                            let packet = Packet::new_fragment(
                                source_routing_header.clone(),
                                session_id,
                                fragment,
                            );

                            // Send the packet to the first hop in the path
                            if let Some(sender) = self.packet_send.get(&path[1]) {
                                match sender.send(packet.clone()) {
                                    Ok(_) => {
                                        // Notify simulation controller
                                        if let Err(e) = self.send_sent_to_sc(packet.clone()) {
                                            debug!("ERROR: Failed to notify SC about sent packet: {:?}", e);
                                        }
                                        debug!("Client: {:?} sent fragment {} of message to {:?}",
                                  self.id, i, node_id);
                                    },
                                    Err(e) => {
                                        debug!("ERROR: Client: {:?} failed to send fragment {} to {:?}: {:?}",
                                  self.id, i, node_id, e);
                                    }
                                }
                            } else {
                                debug!("ERROR: Client: {:?} no sender for node {:?}", self.id, path[1]);
                            }
                        }
                    },
                    Err(e) => {
                        debug!("ERROR: Client: {:?} could not compute path to {:?}: {}", self.id, node_id, e);
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

                let mut new_path: Vec<u8> = _flood_response.path_trace.iter().map(|(id, _)| *id).collect();
                let target_node_id: u8 = new_path.last().unwrap().clone();

                // Update topology map with target as the key
                if !self.topology_map.contains(&(target_node_id, new_path.clone())) {
                    // Case 1: New node entry
                    self.topology_map.insert((target_node_id, new_path));
                    self.add_server_type(target_node_id);
                } else {
                    // Case 2: Existing node - check if new path is better
                    if let Some((_, existing_path)) = self.topology_map.iter().find(|(id, _)| *id == target_node_id) {
                        if _flood_response.path_trace.len() > existing_path.len() {
                            // Replace with better path
                            self.topology_map.remove(&(target_node_id, existing_path.clone()));
                            self.topology_map.insert((target_node_id, new_path));
                            debug!("Client: {:?} received a FloodResponse {:?}", self.id, _flood_response);
                        }
                    }
                };

                debug!("Client: {:?}, updated topology_map: {:?}", self.id, self.topology_map);
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
    fn send_response(&mut self, message: Message<TextRequest>, path: Vec<NodeId>) -> Result<Packet, SendError<Packet>> {
        // create packet
        let target_node_id = 1;
        let source_routing_header = SourceRoutingHeader::new(path, target_node_id);
        let packet = Packet::new_fragment(
            source_routing_header,
            message.session_id,
            Fragment::new(0, 1, [0; 128]), // example data
        );

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
    fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String> {
        // debug!("compute_path_to_node, target_node_id: {:?}", target_node_id);
        // debug!("compute_path_to_node, self.topology_map): {:?}", self.topology_map);

        let path = self
            .topology_map
            .iter()
            .find(|(id, _)| *id == target_node_id);

        match path {
            Some((_, path)) => Ok(path.clone()),
            None => Err("Path not found".to_string()),
        }
    }

    fn add_server_type(&mut self, server_id: NodeId) {
        if !self.server_type_map.contains_key(&server_id){
            //TODO: send request type message
            let server_type = ServerType::Content;
            
            debug!("");
            self.server_type_map.insert(server_id, server_type);
        }
    }
}