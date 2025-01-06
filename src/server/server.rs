use crate::server::assembler::*;
use crate::server::message::*;
use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType,
};

pub struct ContentServer {
    id: NodeId,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ServerEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    assemblers: Vec<Assembler>,
    assembler_send: Sender<Vec<u8>>,
    assembler_recv: Receiver<Vec<u8>>,
}

pub struct CommunicationServer {
    id: NodeId,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ServerEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    assemblers: Vec<Assembler>,
    assembler_send: Sender<Vec<u8>>,
    assembler_recv: Receiver<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub enum ServerType {
    Content,
    CommunicationServer,
}

pub enum ServerEvent {
    PacketSent(Packet),
}

pub trait Server {
    type RequestType: Request;
    type ResponseType: Response;

    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ServerEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        assemblers: Vec<Assembler>,
        topology_map: HashSet<(NodeId, Vec<NodeId>)>,
        assembler_send: Sender<Vec<u8>>,
        assembler_recv: Receiver<Vec<u8>>,
    ) -> Self;

    fn run(&mut self);

    fn send_fragment_to_assembler(&mut self, packet: Packet) -> Result<String, String>;

    fn handle_flood_response(
        &mut self,
        sender_node_id: NodeId,
        flood_response: FloodResponse,
    ) -> Result<String, String>;

    fn send_response(&self, message: Message<TextRequest>) -> Result<Packet, SendError<Packet>>;

    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>>;

    fn compose_message(
        source_id: NodeId,
        session_id: u64,
        raw_content: String,
    ) -> Result<Message<Self::RequestType>, String> {
        let content = Self::RequestType::from_string(raw_content)?;
        Ok(Message {
            session_id,
            source_id,
            content,
        })
    }
    fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String>;
}

impl Server for ContentServer {
    type RequestType = TextRequest;
    type ResponseType = TextResponse;

    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ServerEvent>,
        controller_recv: Receiver<DroneCommand>,
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
                        match command {
                            _ => {
                                return; // TODO handle other commands
                            }
                        }
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        match &packet.pack_type {
                            PacketType::MsgFragment(fragment) => {
                                // handle message fragment
                                let message_fragment_result = self.send_fragment_to_assembler(packet);

                            },
                            PacketType::FloodResponse(flood_response) => {
                                // handle flood request
                                let flood_response_result = self.handle_flood_response(
                                    packet.routing_header.hops[packet.routing_header.hop_index],
                                    flood_response.clone(),
                                );
                                match flood_response_result {
                                    Ok(_) => {},
                                    Err(e) => {
                                        println!("Error: {}", e);
                                    }
                                }
                            },
                            _ => {
                                // ignore
                            }
                        }
                    }
                },
                recv(self.assembler_recv) -> data => {
                    if let Ok(data) = data {
                        // handle assembled data;
                    }
                }
            }
        }
    }

    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>> {
        self.controller_send.send(ServerEvent::PacketSent(packet))
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

    fn send_response(&self, message: Message<TextRequest>) -> Result<Packet, SendError<Packet>> {
        // compute the hops
        let hops: Vec<NodeId> = vec![1, 2, 3];
        // create source header
        let source_routing_header = SourceRoutingHeader::new(hops, 1);

        let packet = Packet::new_fragment(
            source_routing_header,
            message.session_id,
            Fragment::new(0, 1, [0; 128]), // example data
        );

        // send packet
        if let Some(sender) = self.packet_send.get(&message.source_id) {
            // send packet
            match sender.send(packet.clone()) {
                Ok(_) => Ok(packet),
                Err(e) => Err(e),
            }
        } else {
            Err(SendError(packet))
        }
    }

    fn handle_flood_response(
        &mut self,
        sender_node_id: NodeId,
        flood_response: FloodResponse,
    ) -> Result<String, String> {
        // get the path from the flood response
        let mut new_path: Vec<u8> = flood_response
            .path_trace
            .iter()
            .map(|(id, _)| *id)
            .collect();

        // add the current node to the path
        new_path.push(self.id);

        // add the drone to the topology map
        if self.topology_map.insert((sender_node_id, new_path.clone())) {
            return Ok("new node added to topology map".to_string());
        }

        let existing_path_length = self
            .topology_map
            .iter()
            .find(|(id, _)| *id == sender_node_id)
            .unwrap()
            .1
            .len();

        // Check if new vector is longer than existing vector
        if flood_response.path_trace.len() > existing_path_length {
            self.topology_map.insert((sender_node_id, new_path.clone()));
            return Ok("node found in hashset but updated pathtrace".to_string());
        }

        return Err("node already in topology map".to_string());
    }

    fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String> {
        // Map to get path after target found
        let mut came_from: HashMap<NodeId, Option<NodeId>> = HashMap::new();
        let mut queue = VecDeque::new();

        // Make the connected drones the starting nodes
        for &node in self.connected_drone_ids.iter() {
            queue.push_back(node);
            came_from.insert(node, None);
        }

        // Breadth-first search
        while let Some(current) = queue.pop_front() {
            // if target node is found, return the path
            if current == target_node_id {
                let mut path = Vec::new();
                let mut node = Some(current);
                while let Some(n) = node {
                    path.push(n);
                    node = came_from[&n];
                }
                path.reverse(); // get the path from start to finish instead of finish to start
                return Ok(path);
            }

            // Find neighbors of the current node
            if let Some((_, neighbors)) = self.topology_map.iter().find(|(id, _)| *id == current) {
                for &neighbor in neighbors {
                    if !came_from.contains_key(&neighbor) {
                        queue.push_back(neighbor);
                        came_from.insert(neighbor, Some(current));
                    }
                }
            }
        }
        // If we reach this point, the target is unreachable
        Err("Target node is unreachable".to_string())
    }
}
