use std::collections::{HashMap, HashSet};
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodResponse, Fragment, NodeType, Packet};
use crossbeam_channel::{unbounded, SendError, Sender};
use crate::assembler::assembler::Assembler;
use crate::server::message::{DroneSend, Message};

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

pub fn update_topology_with_flood_response(node_id: NodeId, flood_response: &FloodResponse, topology_map: &mut HashSet<(NodeId, Vec<NodeId>)>) {
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
pub fn try_send_packet_with_target_id(id: NodeId, target_node_id: &NodeId, packet: &Packet, packet_send: &HashMap<NodeId, Sender<Packet>>) {
    if let Some(sender) = packet_send.get(&target_node_id) {
        match sender.send(packet.clone()) {
            Ok(_) => {debug!("{:?} -> {:?}\nPacket: {:?}", id, target_node_id, packet);}
            Err(e) => {debug!("ERROR, {:?} -> {:?}\nError: {:?}\nPacket: {:?}", id, target_node_id, e, packet);}
        }
    } else {
        debug!("ERROR, {:?} -> {:?} but {:?} was not found\nPacket: {:?}", id, target_node_id, target_node_id, packet);
    }
}
pub fn try_send_packet(id: NodeId, packet: &Packet, packet_send: &HashMap<NodeId, Sender<Packet>>) {
    if let Some(target_node_id) = packet.routing_header.current_hop() {
        try_send_packet_with_target_id(id, &target_node_id, packet, packet_send);
    }
}
pub fn compute_path_to_node(target_node_id: NodeId, topology_map: &HashSet<(NodeId, Vec<NodeId>)>) -> Result<Vec<NodeId>, String> {
    let path = topology_map.iter().find(|(id, _)| *id == target_node_id);

    match path {
        Some((_, path)) => Ok(path.clone()),
        None => Err("Path not found".to_string()),
    }
}
pub fn send_message_in_fragments<M: DroneSend>(id: NodeId, target_node_id: NodeId, session_id: u64, message: Message<M>, packet_send: &HashMap<NodeId, Sender<Packet>>, topology_map: &HashSet<(NodeId, Vec<NodeId>)>) {
    debug!("Node {:?} sending message to {:?}", id, target_node_id);

    // Serialize the message
    let serialized = serde_json::to_string(&message).unwrap();
    let serialized_bytes = serialized.into_bytes();

    // Calculate fragments needed
    let total_fragments = (serialized_bytes.len() + 127) / 128;

    // Compute path to target
    match compute_path_to_node(target_node_id, &topology_map) {
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

                try_send_packet_with_target_id(id, &path[1], &packet, &packet_send);
            }
        },
        Err(e) => {
            debug!("ERROR: Could not compute path to node {:?}: {}", target_node_id, e);
        }
    }
}
pub fn send_fragment_to_assembler(packet: Packet, assemblers: &mut Vec<Assembler>,) -> Result<String, String> {
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
        Ok(_) => {}
        Err(_) => {
            return Err("Failed to send packet to assembler".to_string());
        }
    }

    // Add new assembler to the list
    assemblers.push(assembler);

    return Ok("Sent fragment to assembler".to_string());
}




