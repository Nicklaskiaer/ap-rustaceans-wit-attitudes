use crate::assembler::assembler::Assembler;
use crate::message::message::{DroneSend, Message, MessageContent};
use crossbeam_channel::{unbounded, Sender};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodResponse, Fragment, NodeType, Packet};

pub enum ClientServerCommand {
    StartFloodRequest, // used by: Client, SText, SMedia, SChat
    RequestServerType, // used by: Client. client will auto call it to itself after few seconds after a StartFloodRequest
    RequestFileList(NodeId), // used by: Client. client ask the server for its list of files
    RequestFile(NodeId, u64), // used by: Client. client ask the server for a specific file
    
    SendChatMessage(NodeId, usize, String),
    // RegistrationRequest(NodeId),
    // RequestServerType(NodeId), // client request the server type
    // ResponseServerType(NodeId), // server send its server type
    // RequestServerList(NodeId), // client request the server a list of all connected clients
    // RequestFileList(NodeId),

    // Drone commands
    DroneCmd(DroneCommand),
}

impl From<DroneCommand> for ClientServerCommand {
    fn from(cmd: DroneCommand) -> Self {
        ClientServerCommand::DroneCmd(cmd)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerType {
    ContentServer(ContentType),
    CommunicationServer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Media,
}

pub enum ServerEvent {
    PacketSent(Packet),
    PacketReceived(Packet),
    MessageSent {
        target: NodeId,
        content: MessageContent,
    },
    MessageReceived {
        content: MessageContent,
    },
}

pub enum ClientEvent {
    PacketSent(Packet),
    PacketReceived(Packet),
    MessageSent {
        target: NodeId,
        content: MessageContent,
    },
    MessageReceived {
        content: MessageContent,
    },
}

pub trait NetworkNode {
    // reference
    fn id(&self) -> NodeId;
    fn packet_send(&self) -> &HashMap<NodeId, Sender<Packet>>;
    fn topology_map(&self) -> &HashSet<(NodeId, Vec<NodeId>)>;
    fn topology_map_mut(&mut self) -> &mut HashSet<(NodeId, Vec<NodeId>)>;
    fn assemblers_mut(&mut self) -> &mut Vec<Assembler>;
    
    
    // common methods to implement
    fn run(&mut self);
    fn send_packet_sent_to_sc(&mut self, packet: Packet);
    fn send_packet_received_to_sc(&mut self, packet: Packet);
    fn send_message_sent_to_sc(&mut self, content: MessageContent, target: NodeId);
    fn send_message_received_to_sc(&mut self, content: MessageContent);
    
    
    // common methods with default implementations
    fn update_topology_with_flood_response(&mut self, flood_response: &FloodResponse) {
        let node_id = self.id();
        let topology_map = self.topology_map_mut();

        // Extract path from path trace
        let new_path: Vec<u8> = flood_response.path_trace.iter().map(|(id, _)| *id).collect();
        let target_node_id: u8 = new_path.last().unwrap().clone();

        // Update topology map with target as the key
        if !topology_map.iter().any(|(id, _)| *id == target_node_id) {
            // Case 1: New node entry - add to topology map
            let &(_, node_type) = flood_response.path_trace.last().unwrap();
            if node_type != NodeType::Drone {
                // Only add servers and clients to topology map
                topology_map.insert((target_node_id, new_path));
                debug!("Node {:?}, updated topology_map: {:?}", node_id, topology_map);
            }
        } else {
            // Case 2: Existing node - check if new path is better
            if let Some((_, existing_path)) = topology_map.iter().find(|(id, _)| *id == target_node_id) {
                if flood_response.path_trace.len() < existing_path.len() {
                    // Replace with better path
                    topology_map.remove(&(target_node_id, existing_path.clone()));
                    topology_map.insert((target_node_id, new_path));
                    debug!("Node {:?}, updated topology_map: {:?}", node_id, topology_map);
                }
            }
        }
    }
    fn try_send_packet_with_target_id(&mut self, target_node_id: &NodeId, packet: &Packet) {
        let id = self.id();
        let packet_send = self.packet_send();

        if let Some(sender) = packet_send.get(target_node_id) {
            match sender.send(packet.clone()) {
                Ok(_) => {
                    debug!("{:?} -> {:?}\nPacket: {:?}", id, target_node_id, packet);
                    self.send_packet_sent_to_sc(packet.clone());
                }
                Err(e) => {debug!("ERROR, {:?} -> {:?}\nError: {:?}\nPacket: {:?}", id, target_node_id, e, packet);}
            }
        } else {
            debug!("ERROR, {:?} -> {:?} but {:?} was not found\nPacket: {:?}", id, target_node_id, target_node_id, packet);
        }
    }
    fn try_send_packet(&mut self, packet: &Packet) {
        if let Some(target_node_id) = packet.routing_header.current_hop() {
            self.try_send_packet_with_target_id(&target_node_id, packet);
        }
    }
    fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String> {
        let topology_map = self.topology_map();
        let path = topology_map.iter().find(|(id, _)| *id == target_node_id);

        match path {
            Some((_, path)) => Ok(path.clone()),
            None => Err("Path not found".to_string()),
        }
    }
    fn send_message_in_fragments<M: DroneSend>(&mut self, target_node_id: NodeId, session_id: u64, message: Message<M>) {
        let id = self.id();
        debug!("Node {:?} sending message to {:?}", id, target_node_id);

        // Serialize the message
        let serialized = serde_json::to_string(&message).unwrap();
        let serialized_bytes = serialized.into_bytes();

        // Calculate fragments needed
        let total_fragments = (serialized_bytes.len() + 127) / 128;

        // Compute path to target
        match self.compute_path_to_node(target_node_id) {
            Ok(path) => {
                // Send fragments
                for i in 0..total_fragments {
                    let start = i * 128;
                    let end = std::cmp::min((i + 1) * 128, serialized_bytes.len());
                    let chunk_size = end - start;

                    let mut data = [0u8; 128];
                    data[..chunk_size].copy_from_slice(&serialized_bytes[start..end]);

                    let fragment = Fragment {
                        fragment_index: i as u64,
                        total_n_fragments: total_fragments as u64,
                        length: chunk_size as u8,
                        data,
                    };

                    let packet = Packet::new_fragment(
                        SourceRoutingHeader::new(path.clone(), 1),
                        session_id,
                        fragment,
                    );

                    self.try_send_packet_with_target_id(&path[1], &packet);
                }

                // Send message sent notification
                if let Some(content) = MessageContent::from_content(message.content) {
                    self.send_message_sent_to_sc(content, target_node_id);
                }
            },
            Err(e) => {
                debug!("ERROR: Could not compute path to node {:?}: {}", target_node_id, e);
            }
        }
    }
    fn send_fragment_to_assembler(&mut self, packet: Packet) -> Result<String, String> {
        let assemblers = self.assemblers_mut();

        // Send the data and the fragment index to the assembler
        for assembler in assemblers.iter_mut() {
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
            Ok(_) => {
                // Add new assembler to the list
                assemblers.push(assembler);
                Ok("Sent fragment to assembler".to_string())
            }
            Err(_) => {
                Err("Failed to send packet to assembler".to_string())
            }
        }
    }
}


























