use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet;
use wg_2024::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType,
};
use crate::assembler::assembler::Assembler;
use crate::server::message::{Message, TextRequest};
use crate::server::server::ServerEvent;

pub struct Client {
    id: NodeId,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ClientEvent>,
    controller_recv: Receiver<DroneCommand>,
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

    fn send_response(&mut self, message: Message<TextRequest>)
        -> Result<Packet, SendError<Packet>>;

    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ClientEvent>>;
    fn send_recv_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ClientEvent>>;

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
    fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String>;
}

impl ClientTrait for Client {
    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ClientEvent>,
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
                                // send received packet to simulation controller
                                let sc_send_res = self.send_recv_to_sc(packet.clone());
                                match sc_send_res {
                                    Ok(_) => {},
                                    Err(e) => {
                                        println!("Error: {}", e);
                                    }
                                }
                                // handle message fragment
                                let message_fragment_result = self.send_fragment_to_assembler(packet);
                                match message_fragment_result {
                                    Ok(_) => {},
                                    Err(e) => {
                                        println!("Error: {}", e);
                                    }
                                }
                            },
                            PacketType::FloodResponse(flood_response) => {
                                // send received packet to simulation controller
                                let sc_send_res = self.send_recv_to_sc(packet.clone());
                                match sc_send_res {
                                    Ok(_) => {},
                                    Err(e) => {
                                        println!("Error: {}", e);
                                    }
                                }
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
                            PacketType::Ack(_) => {
                                // handle ack
                            },
                            PacketType::Nack(_) => {
                                // handle nack
                            },
                            PacketType::FloodRequest(_) => {
                                // handle flood request
                            },
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

    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ClientEvent>> {
        self.controller_send.send(ClientEvent::PacketSent(packet))
    }

    fn send_recv_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ClientEvent>> {
        self.controller_send
            .send(ClientEvent::PacketReceived(packet))
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

    fn send_response(
        &mut self,
        message: Message<TextRequest>,
    ) -> Result<Packet, SendError<Packet>> {
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
                            println!("Error: {}", e);
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

    // find the route to the node in the hashmap, and return the path
    fn compute_path_to_node(&self, target_node_id: NodeId) -> Result<Vec<NodeId>, String> {
        let path = self
            .topology_map
            .iter()
            .find(|(id, _)| *id == target_node_id);

        match path {
            Some((_, path)) => Ok(path.clone()),
            None => Err("Path not found".to_string()),
        }
    }
}
