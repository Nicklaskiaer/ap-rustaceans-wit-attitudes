use crate::server::message::*;
use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use rand::{random, Rng};
use std::collections::HashMap;
use std::{fs, thread};
use wg_2024::network::*;

use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::Nack;
use wg_2024::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType,
};

pub struct ContentServer {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
}
pub struct CommunicationServer {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
}

#[derive(Debug, Clone)]
pub enum ServerType {
    Content,
    CommunicationServer,
}

pub trait Server {
    type RequestType: Request;
    type ResponseType: Response;

    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
    ) -> Self;
    fn run(&mut self);
    // fn handle_command() -> Result<(), String>;
    fn get_server_type(&self) -> ServerType;
    fn handle_request(&self, message: Message<TextRequest>) -> Message<TextRequest>;
    fn send_response(&self, message: Message<TextRequest>) -> Result<Packet, SendError<Packet>>;
    // fn compute_route(&self, hops: Vec<i32>, hop_index: usize) -> Vec<i32>;
    fn create_source_routing_header(
        &self,
        hops: Vec<NodeId>,
        hop_index: usize,
    ) -> SourceRoutingHeader;
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
}

impl Server for ContentServer {
    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
    ) -> Self {
        Self {
            id,
            connected_drone_ids,
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
        }
    }

    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        // if let DroneCommand::Crash = command {
                        //     println!("drone {} crashed", self.id);
                        //     break;
                        // }
                        // self.handle_command(command);
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        match packet.pack_type {
                            PacketType::MsgFragment(fragment) => {
                                // handle message
                                let message = Message {
                                    source_id: packet.routing_header.hops[packet.routing_header.hop_index],
                                    session_id: packet.session_id,
                                    content: TextRequest::from_string(String::from_utf8_lossy(&fragment.data).to_string()).unwrap(),
                                };
                                let response = self.handle_request(message);
                                let response_packet = self.send_response(response);
                                // send response
                                match response_packet {
                                    Ok(packet) => {
                                        if let Some(sender) = self.packet_send.get(&packet.routing_header.hops[packet.routing_header.hop_index]) {
                                            match sender.send(packet) {
                                                Ok(_) => {},
                                                Err(e) => {
                                                    println!("Error sending packet: {:?}", e);
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        println!("Error sending packet: {:?}", e);
                                    }
                                }
                            },
                            PacketType::Ack(ack) => {
                                // handle ack
                            },
                            PacketType::Nack(nack) => {
                                // handle nack
                            },
                            PacketType::FloodRequest(flood_request) => {
                                // handle flood request
                            },
                            PacketType::FloodResponse(flood_response) => {
                                // handle flood response
                            },
                        }
                    }
                },
            }
        }
    }

    type RequestType = TextRequest;
    type ResponseType = TextResponse;

    fn get_server_type(&self) -> ServerType {
        ServerType::Content
    }

    fn handle_request(&self, message: Message<TextRequest>) -> Message<TextRequest> {
        match message.content {
            TextRequest::TextList => {
                let response = TextRequest::Text(1);
                Message {
                    source_id: message.source_id,
                    session_id: message.session_id,
                    content: response,
                }
            }
            TextRequest::Text(_) => {
                let response = TextRequest::Text(1);
                Message {
                    source_id: message.source_id,
                    session_id: message.session_id,
                    content: response,
                }
            }
        }
    }

    fn send_response(&self, message: Message<TextRequest>) -> Result<Packet, SendError<Packet>> {
        // compute the route
        // let hos = Self::compute_route();
        let hops: Vec<NodeId> = vec![1, 2, 3];
        // create source header
        let source_routing_header = self.create_source_routing_header(hops, 1);
        // create packet
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

    fn create_source_routing_header(
        &self,
        hops: Vec<NodeId>,
        hop_index: usize,
    ) -> SourceRoutingHeader {
        SourceRoutingHeader::new(hops, hop_index)
    }
}
