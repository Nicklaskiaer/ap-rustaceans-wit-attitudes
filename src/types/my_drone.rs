#![allow(unused)]

use crossbeam_channel::{select_biased, unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::{fs, thread};
use rand::{random, Rng};

use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType};
use wg_2024::packet::Nack;


pub struct MyDrone {
    id: NodeId,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
}

impl Drone for MyDrone {
    fn new(
        id: NodeId,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        pdr: f32,
    ) -> Self {
        Self {
            id,
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
            pdr,
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
                        self.handle_command(command);
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.handle_packet(packet);
                    }
                },
            }
        }
    }
}

impl MyDrone {
    // <editor-fold desc="Simulation controller commands">
    fn handle_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::SetPacketDropRate(_pdr) =>{self.pdr = _pdr},
            DroneCommand::Crash => {self.crash()},
            DroneCommand::AddSender(_node_id, _sender) => {self.add_sender(_node_id, _sender)},
            DroneCommand::RemoveSender(_node_id) => {self.remove_sender(_node_id)},
        }
    }
    fn crash(&mut self){
        //TODO: make the crash
    }
    fn add_sender(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
    fn remove_sender(&mut self, id: NodeId) {
        self.packet_send.remove(&id);
    }
    // </editor-fold>

    // <editor-fold desc="Packets">
    fn handle_packet(&mut self, mut packet: Packet) {
        let mut new_hop_index: usize = 0;

        // check for UnexpectedRecipient (will send the package backwards)
        if self.id != packet.routing_header.hops[packet.routing_header.hop_index] {
            self.send_nack(
                packet,
                NackType::UnexpectedRecipient(self.id)
            );
            return;
        }

        // check for DestinationIsDrone (will send the package backwards)
        if packet.routing_header.hops.len() == packet.routing_header.hop_index {
            let p = self.send_nack(
                packet,
                NackType::DestinationIsDrone,
            );
            return;
        }

        // check for ErrorInRouting (will send the package backwards)
        if !self.packet_send.contains_key(&packet.routing_header.hops[packet.routing_header.hop_index + 1]) {
            let p = self.send_nack(
                packet.clone(),
                NackType::ErrorInRouting(packet.routing_header.hops[packet.routing_header.hop_index + 1]),
            );
            return;
        }

        // match with all Packet Types
        match packet.clone().pack_type {
            PacketType::Nack(_nack) => {
                self.send_nack(
                    packet,
                    _nack.nack_type
                );
                return;
            }
            PacketType::Ack(_ack) => {
                self.send_ack(
                    packet,
                    _ack
                );
                return;
            }
            PacketType::MsgFragment(_fragment) => {
                // check if it's Dropped
                let mut rng = rand::thread_rng();
                if rng.gen_range(0.0..=1.0) < self.pdr {
                    self.send_nack(
                        packet,
                        NackType::Dropped
                    );
                    // continue to send the inverted hop_index
                    // let next_node_id = p.routing_header.hops[p.routing_header.hop_index-1];
                    // if let Some(drone) = self.packet_send.get(&next_node_id).cloned() {
                    //     self.send_packet(p, vec![drone]);
                    // }
                    return;
                } else {
                    // send fragment
                    self.send_msg_fragment(
                        packet,
                        _fragment
                    );
                    return;
                }
            }
            PacketType::FloodRequest(_flood_request) => {
                // is it the first time the node receives this flood request?
                if !_flood_request.path_trace.contains(&(self.id, NodeType::Drone)) {
                    // yes: send a flood request to all neighbors
                    self.send_flood_request(
                        packet,
                        _flood_request
                    );
                    return;
                } else {
                    // no: send a flood response
                    let flood_response = FloodResponse{
                        flood_id: _flood_request.flood_id,
                        path_trace: _flood_request.path_trace,
                    };
                    self.send_flood_response(packet, flood_response);
                    return;
                }
            },
            PacketType::FloodResponse(_flood_response) => {
                self.send_flood_response(
                    packet,
                    _flood_response
                );
                return;
            },
        }
    }
    fn send_nack(&mut self, packet: Packet, nack_type: NackType){
        let next_hop_index = packet.routing_header.hop_index - 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

        if let Some(sender) = self.packet_send.get(&next_node_id) {
            // generate new packet
            let p = Packet {
                pack_type: PacketType::Nack(Nack {
                    fragment_index: match packet.pack_type {
                        PacketType::MsgFragment(_fragment) => _fragment.fragment_index,
                        _ => 0,
                    },
                    nack_type,
                }),
                routing_header: SourceRoutingHeader {
                    hop_index: next_hop_index,
                    hops: packet.routing_header.hops.clone(),
                },
                session_id: packet.session_id,
            };

            // send packet
            sender.send(p).unwrap();
        } else {
            panic!("Sender not found, cannot send: {:?}", nack_type);
        }
        return;
    }
    fn send_ack(&mut self, packet: Packet, ack: Ack){
        let next_hop_index = packet.routing_header.hop_index - 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

        if let Some(sender) = self.packet_send.get(&next_node_id) {
            // generate new packet
            let p = Packet {
                pack_type: PacketType::Ack(Ack {
                    fragment_index: ack.fragment_index,
                }),
                routing_header: SourceRoutingHeader {
                    hop_index: next_hop_index,
                    hops: packet.routing_header.hops
                },
                session_id: packet.session_id,
            };

            // send packet
            sender.send(p).unwrap();
        } else {
            panic!("Sender not found, cannot send: {:?}", ack);
        }
        return;
    }
    fn send_msg_fragment(&mut self, packet: Packet, fragment: Fragment){
        let next_hop_index = packet.routing_header.hop_index + 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

        if let Some(sender) = self.packet_send.get(&next_node_id) {
            // generate new packet
            let p = Packet {
                pack_type: PacketType::MsgFragment(Fragment{
                    fragment_index: fragment.fragment_index,
                    total_n_fragments: fragment.total_n_fragments,
                    length: fragment.length,
                    data: fragment.data,
                }),
                routing_header: SourceRoutingHeader {
                    hop_index: next_hop_index,
                    hops: packet.routing_header.hops,
                },
                session_id: packet.session_id,
            };

            // send packet
            sender.send(p).unwrap();
        } else {
            panic!("Sender not found, cannot send: {:?}", fragment);
        }
        return;
    }
    fn send_flood_request(&mut self, packet: Packet, flood_request: FloodRequest){
        let next_hop_index = packet.routing_header.hop_index + 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

        // add node to the path trace
        let mut new_path_trace = flood_request.path_trace;
        new_path_trace.push((self.id, NodeType::Drone));

        // generate new packet
        let p = Packet {
            pack_type: PacketType::FloodRequest(FloodRequest{
                flood_id: flood_request.flood_id,
                initiator_id: flood_request.initiator_id,
                path_trace: new_path_trace,
            }),
            routing_header: SourceRoutingHeader {
                hop_index: next_hop_index,
                hops: packet.routing_header.hops.clone(),
            },
            session_id: packet.session_id,
        };

        // send packet to neighbors (except for the previous drone)
        let prev_node_id = packet.routing_header.hops[packet.routing_header.hop_index-1];
        for sender in &self.packet_send{
            if *sender.0 != prev_node_id{
                sender.1.send(p.clone()).unwrap();
            }
        }
        return;
    }
    fn send_flood_response(&mut self, packet: Packet, flood_response: FloodResponse){
        let next_hop_index = packet.routing_header.hop_index - 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

        if let Some(sender) = self.packet_send.get(&next_node_id) {
            // generate new packet
            let p = Packet {
                pack_type: PacketType::FloodResponse(FloodResponse{
                    flood_id: flood_response.flood_id,
                    path_trace: flood_response.path_trace,
                }),
                routing_header: SourceRoutingHeader {
                    hop_index: next_hop_index,
                    hops: packet.routing_header.hops,
                },
                session_id: packet.session_id,
            };

            // send packet
            sender.send(p).unwrap();
        } else {
            panic!("Sender not found, cannot send: {:?}", flood_response);
        }
        return;
    }
    // </editor-fold>
}
