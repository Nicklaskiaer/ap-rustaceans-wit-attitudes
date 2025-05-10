#[cfg(feature = "debug")]
use crate::debug;

use crate::assembler::assembler::*;
use crate::server::message::*;
use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use rand::random;
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet;
use wg_2024::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType,
};
use crate::client::client::Client;
use crate::client::client_server_command::ClientServerCommand;

pub struct ContentServer {
    id: NodeId,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ServerEvent>,
    controller_recv: Receiver<ClientServerCommand>,
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
    controller_recv: Receiver<ClientServerCommand>,
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
    PacketReceived(Packet),
}

pub trait Server {
    type RequestType: Request;
    type ResponseType: Response;

    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ServerEvent>,
        controller_recv: Receiver<ClientServerCommand>,
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

    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>>;
    fn send_recv_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>>;

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

    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>> {
        self.controller_send.send(ServerEvent::PacketSent(packet))
    }
    fn send_recv_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>> {
        self.controller_send
            .send(ServerEvent::PacketReceived(packet))
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
        let target_node_id = 1;
        let mut hops = Vec::new();
        if let Ok(computed_hops) = self.compute_path_to_node(target_node_id) {
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
    fn handle_flood_response(&mut self, sender_node_id: NodeId, flood_response: FloodResponse) -> Result<String, String> {
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

impl ContentServer {
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
            ClientServerCommand::StartFloodRequest => {
                debug!("Server: {:?} received StartFloodRequest command", self.id);
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
                debug!("Server: {:?} received SendChatMessage command for node {:?}: {:?}", self.id, node_id, msg);
            },
            // ClientServerCommand::RequestServerType(_) => {
            //     debug!("Server: {:?} received RequestServerType command", self.id);
            // },
            // ClientServerCommand::ResponseServerType(_) => {
            //     debug!("Server: {:?} received ResponseServerType command", self.id);
            // }
        }
    }
    fn handle_packet(&mut self, mut packet: Packet) {
        match &packet.pack_type {
            PacketType::MsgFragment(_fragment) => {
                debug!("Server: {:?} received a MsgFragment {:?}", self.id, _fragment);

                // Send to assembler
                if let Ok(_) = self.send_fragment_to_assembler(packet.clone()) {
                    // Check if this is the last/only fragment
                    if _fragment.fragment_index == _fragment.total_n_fragments - 1 {
                        // Try to process after assembly is complete
                        if let Ok(data) = self.assembler_recv.try_recv() {
                            if let Ok(str_data) = std::str::from_utf8(&data) {
                                debug!("Assembled message: {}", str_data);
                                println!("Server {} received assembled message: {}", self.id, str_data);
                            }
                        }
                    }
                }
                
                
                
                
                // // send received packet to simulation controller
                // let sc_send_res = self.send_recv_to_sc(packet.clone());
                // match sc_send_res {
                //     Ok(_) => {},
                //     Err(e) => {
                //         // println!("Error: {}", e);
                //     }
                // }
                // // handle message fragment
                // let message_fragment_result = self.send_fragment_to_assembler(packet);
                // match message_fragment_result {
                //     Ok(_) => {},
                //     Err(e) => {
                //         // println!("Error: {}", e);
                //     }
                // }
            },
            PacketType::Ack(_ack) => {
                debug!("Server: {:?} received a Ack {:?}", self.id, _ack);
                // handle ack
            },
            PacketType::Nack(_Nack) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _Nack);
                // handle nack
            },
            PacketType::FloodRequest(_FloodRequest) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _FloodRequest);
            },
            PacketType::FloodResponse(_flood_response) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _flood_response);
            },
        }
    }
}




impl Server for CommunicationServer {
    type RequestType = TextRequest;
    type ResponseType = TextResponse;

    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ServerEvent>,
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

    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>> {
        self.controller_send.send(ServerEvent::PacketSent(packet))
    }
    fn send_recv_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>> {
        self.controller_send
            .send(ServerEvent::PacketReceived(packet))
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
        let target_node_id = 1;
        let mut hops = Vec::new();
        if let Ok(computed_hops) = self.compute_path_to_node(target_node_id) {
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
    fn handle_flood_response(&mut self, sender_node_id: NodeId, flood_response: FloodResponse) -> Result<String, String> {
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

impl CommunicationServer {
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
            ClientServerCommand::StartFloodRequest => {
                debug!("Server: {:?} received StartFloodRequest command", self.id);
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
                debug!("Server: {:?} received SendChatMessage command for node {:?}: {:?}", self.id, node_id, msg);
            },
            // ClientServerCommand::RequestServerType(_) => {
            //     debug!("Server: {:?} received RequestServerType command", self.id);
            // },
            // ClientServerCommand::ResponseServerType(_) => {
            //     debug!("Server: {:?} received ResponseServerType command", self.id);
            // }
        }
    }
    fn handle_packet(&mut self, mut packet: Packet) {
        match &packet.pack_type {
            PacketType::MsgFragment(_fragment) => {
                debug!("Server: {:?} received a MsgFragment {:?}", self.id, _fragment);

                // Send to assembler
                if let Ok(_) = self.send_fragment_to_assembler(packet.clone()) {
                    // Check if this is the last/only fragment
                    if _fragment.fragment_index == _fragment.total_n_fragments - 1 {
                        // Try to process after assembly is complete
                        if let Ok(data) = self.assembler_recv.try_recv() {
                            if let Ok(str_data) = std::str::from_utf8(&data) {
                                debug!("Assembled message: {}", str_data);
                                println!("Server {} received assembled message: {}", self.id, str_data);
                            }
                        }
                    }
                }




                // // send received packet to simulation controller
                // let sc_send_res = self.send_recv_to_sc(packet.clone());
                // match sc_send_res {
                //     Ok(_) => {},
                //     Err(e) => {
                //         // println!("Error: {}", e);
                //     }
                // }
                // // handle message fragment
                // let message_fragment_result = self.send_fragment_to_assembler(packet);
                // match message_fragment_result {
                //     Ok(_) => {},
                //     Err(e) => {
                //         // println!("Error: {}", e);
                //     }
                // }
            },
            PacketType::Ack(_ack) => {
                debug!("Server: {:?} received a Ack {:?}", self.id, _ack);
                // handle ack
            },
            PacketType::Nack(_nack) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _nack);
                // handle nack
            },
            PacketType::FloodRequest(_flood_request) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _flood_request);

                // send a flood response
                // add node to the path trace
                let mut flood_request = _flood_request.clone();
                flood_request.increment(self.id, NodeType::Drone);
                // generate a flood response
                let mut flood_response_packet = flood_request.generate_response(packet.session_id);
                debug!("Server: {:?} is generating a flood_request: {:?}", self.id, flood_response_packet);
                flood_response_packet.routing_header.increase_hop_index();

                // Try to send packet
                match flood_response_packet.routing_header.current_hop() {
                    None => {panic!("*surprised quack*, Server: {:?} pack: {:?}", self.id, flood_response_packet)}
                    Some(_next_node_id) => { self.try_send_packet(flood_response_packet, _next_node_id).expect("TODO: panic message");}
                }
            },
            PacketType::FloodResponse(_flood_response) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _flood_response);
            },
        }
    }
    
    fn try_send_packet(&self, p: Packet, next_node_id: NodeId) -> Result<Packet, SendError<Packet>> {
        if let Some(sender) = self.packet_send.get(&next_node_id) {
            // send packet
            match sender.send(p.clone()) {
                Ok(_) => {
                    debug!("Drone: {:?} sent packet {:?} to {:?}", self.id, p.pack_type, next_node_id);
                    Ok(p)
                },
                Err(e) => Err(e),
            }
        } else {
            debug!("ERROR, Sender not found, Drone: {:?} cannot send Packet to: {:?}\nPacket: {:?}", self.id, next_node_id, p);
            Err(SendError(p))
        }
    }
}