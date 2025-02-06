#![allow(unused)]

use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
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
    controller_send: Sender<DroneEvent>,        // send to sc
    controller_recv: Receiver<DroneCommand>,    // receive from sc
    packet_recv: Receiver<Packet>,              // receive to neighbor nodes
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,   // send to neighbor nodes
    flood_initiators: HashMap<u64, NodeId>,
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
            flood_initiators: HashMap::new()
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
        // While in this loop the drone is in crashing state
        println!("{} in crashing state", self.id);
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        match command {
                            // If no senders are left, the drone can exit the crashing state and be considered as crashed
                            DroneCommand::RemoveSender(_node_id) => {
                                self.remove_sender(_node_id);
                                if self.packet_send.is_empty() {
                                    println!("{} finally crashed", self.id);
                                    return;
                                }
                            }
                            
                            // Ignore other commands while crashing
                            _ => {}
                        }
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        match packet.pack_type.clone() {
                            // Lose FloodRequest
                            PacketType::FloodRequest(_) => {
                                // Do nothing 
                            }
                
                            // Forward Ack, Nack, and FloodResponse
                            PacketType::Ack(_ack) => {
                                match self.send_ack(packet.clone(), _ack){
                                    Err(p) => {self.send_shortcut_to_sc(p.0)}
                                    _ => {}
                                }
                            }
                            PacketType::Nack(_nack) => {
                                match self.send_nack(packet.clone(), _nack.nack_type){
                                    Err(p) => {self.send_shortcut_to_sc(p.0)}
                                    _ => {}
                                }
                            }
                            PacketType::FloodResponse(_flood_response) => {
                                match self.send_flood_response(packet, _flood_response){
                                    Err(p) => {self.send_shortcut_to_sc(p.0)}
                                    _ => {}
                                }
                            }
                
                            // Send Nack(ErrorInRouting) for other packet types
                            PacketType::MsgFragment(_) => {
                                self.send_nack(
                                    packet,
                                    NackType::ErrorInRouting(self.id)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    fn add_sender(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
    fn remove_sender(&mut self, id: NodeId) {
        self.packet_send.remove(&id);
        println!("{} removed {}", self.id, id);
    }
    fn send_dropped_to_sc(&mut self, packet: Packet){
        self.controller_send.send(DroneEvent::PacketDropped(packet));
    }
    fn send_sent_to_sc(&mut self, packet: Packet){
        self.controller_send.send(DroneEvent::PacketSent(packet));
    }
    fn send_shortcut_to_sc(&mut self, packet: Packet){
        self.controller_send.send(DroneEvent::ControllerShortcut(packet));
    }
    // </editor-fold>

    // <editor-fold desc="Packets">
    fn handle_packet(&mut self, mut packet: Packet) {

        // first thing first check if it's a FloodRequest 
        // if so, hop_index and hops will be ignored
        if !matches!(packet.pack_type, PacketType::FloodRequest(_)){
            
            // check for UnexpectedRecipient (will send the package backwards)
            if self.id != packet.routing_header.hops[packet.routing_header.hop_index]{
                let p = self.send_nack(
                    packet.clone(),
                    NackType::UnexpectedRecipient(self.id)
                );
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }

            // check for DestinationIsDrone (will send the package backwards)
            if packet.routing_header.hops.len() == packet.routing_header.hop_index {
                let p = self.send_nack(
                    packet.clone(),
                    NackType::DestinationIsDrone
                );
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }

            // check for ErrorInRouting (will send the package backwards)
            if !self.packet_send.contains_key(&packet.routing_header.hops[packet.routing_header.hop_index + 1]) {
                let p = self.send_nack(
                    packet.clone(),
                    NackType::ErrorInRouting(packet.routing_header.hops[packet.routing_header.hop_index + 1]),
                );
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }
        }

        // match with all Packet Types
        match packet.clone().pack_type {
            PacketType::Nack(_nack) => {
                let p = self.send_nack(
                    packet.clone(),
                    _nack.nack_type
                );
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }
            PacketType::Ack(_ack) => {
                let p = self.send_ack(
                    packet.clone(),
                    _ack
                );
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }
            PacketType::MsgFragment(_fragment) => {
                // check if it's Dropped
                let mut rng = rand::thread_rng();
                if rng.gen_range(0.0..=1.0) < self.pdr {
                    let p = self.send_nack(
                        packet.clone(),
                        NackType::Dropped
                    );
                    match p{
                        Ok(_p) => {self.send_dropped_to_sc(_p)} // send the dropped packet to the simulation controller
                        Err(_p) => {panic!("*surprised quack*")} // self.send_shortcut_to_sc(_p.0)
                    }
                    return;
                } else {
                    // send fragment
                    let p =self.send_msg_fragment(
                        packet.clone(),
                        _fragment
                    );
                    match p{
                        Ok(_p) => {self.send_sent_to_sc(_p)}
                        Err(_p) => {panic!("*surprised quack*")} //self.send_shortcut_to_sc(_p.0)
                    }
                    return;
                }
            }
            PacketType::FloodRequest(_flood_request) => {
                // is it the first time the node receives this flood request?
                let current_flood = self.flood_initiators.get_key_value(&_flood_request.flood_id);
                let drone_flood = (&_flood_request.flood_id, &_flood_request.initiator_id);
                if current_flood.is_none() || current_flood.unwrap() != drone_flood{
                    // yes: send a flood request to all neighbors
                    let p = self.send_flood_request(
                        packet.clone(),
                        _flood_request
                    );
                    match p{
                        Ok(_p) => {self.send_sent_to_sc(_p)}
                        Err(_p) => {panic!("*surprised quack*")} //self.send_shortcut_to_sc(_p.0)
                    }
                    return;
                } else {
                    // no: send a flood response
                    let flood_response = FloodResponse{
                        flood_id: _flood_request.flood_id,
                        path_trace: _flood_request.path_trace,
                    };
                    let p = self.send_flood_response(packet.clone(), flood_response);
                    match p{
                        Ok(_p) => {self.send_sent_to_sc(_p)}
                        Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                    }
                    return;
                }
            },
            PacketType::FloodResponse(_flood_response) => {
                let p = self.send_flood_response(
                    packet.clone(),
                    _flood_response
                );
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            },
        }
    }
    fn send_nack(&mut self, packet: Packet, nack_type: NackType)->Result<(Packet), SendError<Packet>>{
        let next_hop_index = packet.routing_header.hop_index - 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

        // generate new packet
        let p = Packet {
            pack_type: PacketType::Nack(Nack {
                fragment_index: match packet.pack_type {
                    PacketType::MsgFragment(_fragment) => _fragment.fragment_index,
                    PacketType::Nack(_nack) => _nack.fragment_index,
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

        // try to send packet
        self.try_send_packet(p, next_node_id)
    }
    fn send_ack(&mut self, packet: Packet, ack: Ack)->Result<(Packet), SendError<Packet>>{
        let next_hop_index = packet.routing_header.hop_index - 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

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

        // try to send packet
        self.try_send_packet(p, next_node_id)
    }
    fn send_msg_fragment(&mut self, packet: Packet, fragment: Fragment)->Result<(Packet), SendError<Packet>>{
        let next_hop_index = packet.routing_header.hop_index + 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

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

        // try to send packet
        self.try_send_packet(p, next_node_id)
    }
    fn send_flood_request(&mut self, packet: Packet, flood_request: FloodRequest)->Result<(Packet), SendError<Packet>>{
        let next_hop_index = packet.routing_header.hop_index + 1;

        // add node to the hops
        let mut new_hops = packet.routing_header.hops.clone();
        new_hops.push((self.id));
        
        // add node to the path trace
        let mut new_path_trace = flood_request.path_trace.clone();
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
                hops: new_hops,
            },
            session_id: packet.session_id,
        };

        // send packet to neighbors (except for the previous drone)
        let prev_node_id = packet.routing_header.hops[packet.routing_header.hop_index-1];
        for (node_id, _) in self.packet_send.clone(){
            if prev_node_id != node_id{
                // try to send packet
                match self.try_send_packet(p.clone(), node_id){
                    Ok(_) => {}
                    Err(e) => {return Err(e)}
                }
            }
        }
        Ok(p)
    }
    fn send_flood_response(&mut self, packet: Packet, flood_response: FloodResponse)->Result<(Packet), SendError<Packet>>{
        let next_hop_index = packet.routing_header.hop_index - 1;
        let next_node_id = packet.routing_header.hops[next_hop_index];

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

        // try to send packet
        self.try_send_packet(p, next_node_id)
    }
    fn try_send_packet(&self, p: Packet, next_node_id: NodeId) -> Result<Packet, SendError<Packet>> {
        if let Some(sender) = self.packet_send.get(&next_node_id) {
            // send packet
            match sender.send(p.clone()) {
                Ok(_) => Ok(p),
                Err(e) => Err(e),
            }
        } else {
            Err(panic!("Sender not found, cannot send: {:?}", p))
        }
    }
    // </editor-fold>
}
