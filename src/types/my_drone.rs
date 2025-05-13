#![allow(unused)]

#[cfg(feature = "debug")]
use crate::debug;

use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use std::collections::HashMap;
use std::{fs, thread};
use std::any::Any;
use rand::{random, Rng};

use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType};
use wg_2024::packet::Nack;
use wg_2024::packet::PacketType::MsgFragment;

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
                        self.handle_command(command);
                    }
                },
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
            DroneCommand::SetPacketDropRate(_pdr) =>{
                debug!("Drone: {:?} received command SetPacketDropRate", self.id);
                debug!("Drone: {:?} changed pdf from {:?} to {:?}", self.id, self.pdr, _pdr);
                self.pdr = _pdr
            },
            DroneCommand::Crash => {
                debug!("Drone: {:?} received command Crash", self.id);
                self.crash()
            },
            DroneCommand::AddSender(_node_id, _sender) => {
                debug!("Drone: {:?} received command AddSender", self.id);
                self.add_sender(_node_id, _sender)
            },
            DroneCommand::RemoveSender(_node_id) => {
                debug!("Drone: {:?} received command RemoveSender", self.id);
                self.remove_sender(_node_id)
            },
        }
    }
    fn crash(&mut self){
        debug!("Drone: {:?} is in crashing state", self.id);
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        match command {
                            // If no senders are left, the drone can exit the crashing state and be considered as crashed
                            DroneCommand::RemoveSender(_node_id) => {
                                self.remove_sender(_node_id);
                                if self.packet_send.is_empty() {
                                    debug!("Drone: {:?} completed the crash", self.id);
                                    return;
                                }
                            }

                            // Ignore other commands while crashing
                            _ => {}
                        }
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(mut packet) = packet {
                        debug!("Drone: {:?} received packet {:?} while in crashing state", self.id, packet.pack_type);
                        match packet.pack_type.clone() {
                            // Lose FloodRequest
                            PacketType::FloodRequest(_) => {
                                // Do nothing
                            }

                            // Forward Ack, Nack, and FloodResponse
                            PacketType::Ack(_) => {
                                let p = self.forward_packet(packet);
                                match p{
                                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                                    _ => {}
                                }
                                return;
                            } 
                            PacketType::Nack(_) => {
                                let p = self.forward_packet(packet);
                                match p{
                                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                                    _ => {}
                                }
                                return;
                            }
                            PacketType::FloodResponse(_) => {
                                let p = self.forward_packet(packet);
                                match p{
                                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                                    _ => {}
                                }
                                return;
                            }

                            // Send Nack(ErrorInRouting) for other packet types
                            PacketType::MsgFragment(_) => {
                                packet.routing_header.reverse();
                                let new_packet = Packet::new_nack(
                                    packet.routing_header.clone(),
                                    packet.session_id,
                                    Nack{
                                        fragment_index: packet.get_fragment_index(),
                                        nack_type: NackType::ErrorInRouting(self.id)
                                    }
                                );
                                let p = self.forward_packet(new_packet);
                                match p{
                                    Ok(_p) => {self.send_sent_to_sc(_p)}
                                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                                }
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    fn add_sender(&mut self, id: NodeId, sender: Sender<Packet>) {
        debug!("Drone: {:?} add sender {:?}", self.id, id);
        self.packet_send.insert(id, sender);
    }
    fn remove_sender(&mut self, id: NodeId) {
        debug!("Drone: {:?} remove sender {:?}", self.id, id);
        self.packet_send.remove(&id);
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
        debug!("Drone: {:?} received packet {:?}", self.id, packet.pack_type);

        // first thing first check if it's a FloodRequest
        // if so, hop_index and hops will be ignored
        if !matches!(packet.pack_type, PacketType::FloodRequest(_)){
            
            // check for UnexpectedRecipient (will send the package backwards)
            if self.id != packet.routing_header.current_hop().unwrap(){
                debug!("Drone: {:?} got UnexpectedRecipient error", self.id);
                packet.routing_header.reverse();
                let new_packet = Packet::new_nack(
                    packet.routing_header.clone(),
                    packet.session_id,
                    Nack{
                        fragment_index: packet.get_fragment_index(),
                        nack_type: NackType::UnexpectedRecipient(self.id)
                    }
                );
                let p = self.forward_packet(new_packet);
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }

            // check for DestinationIsDrone (will send the package backwards)
            if packet.routing_header.hops.len() == packet.routing_header.hop_index {
                debug!("Drone: {:?} got DestinationIsDrone error", self.id);
                packet.routing_header.reverse();
                let new_packet = Packet::new_nack(
                    packet.routing_header.clone(),
                    packet.session_id,
                    Nack{
                        fragment_index: packet.get_fragment_index(),
                        nack_type: NackType::DestinationIsDrone
                    }
                );
                let p = self.forward_packet(new_packet);
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }

            // check for ErrorInRouting (will send the package backwards)
            if !self.packet_send.contains_key(&packet.routing_header.hops[packet.routing_header.hop_index + 1]) {
                debug!("Drone: {:?} got ErrorInRouting error", self.id);
                packet.routing_header.reverse();
                let new_packet = Packet::new_nack(
                    packet.routing_header.clone(),
                    packet.session_id,
                    Nack{
                        fragment_index: packet.get_fragment_index(),
                        nack_type: NackType::ErrorInRouting(self.id)
                    }
                );
                let p = self.forward_packet(new_packet);
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }
        }

        // match with all Packet Types
        match packet.clone().pack_type {
            PacketType::Nack(_) => {
                let p = self.forward_packet(packet);
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            }
            PacketType::Ack(_) => {
                let p = self.forward_packet(packet);
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            } 
            PacketType::MsgFragment(_) => {
                // check if it's Dropped
                let mut rng = rand::thread_rng();
                if rng.gen_range(0.0..=1.0) < self.pdr {
                    // forward Dropped
                    packet.routing_header.reverse();
                    let new_packet = Packet::new_nack(
                        packet.routing_header.clone(),
                        packet.session_id,
                        Nack{ 
                            fragment_index: packet.get_fragment_index(), 
                            nack_type: NackType::Dropped 
                        }
                    );
                    let p = self.forward_packet(new_packet);
                    match p{
                        Ok(_p) => {self.send_sent_to_sc(_p)}
                        Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                    }
                    return;
                } else {
                    // forward fragment
                    let p = self.forward_packet(packet);
                    match p{
                        Ok(_p) => {self.send_sent_to_sc(_p)}
                        Err(_p) => {panic!("*surprised quack*")}
                    }
                    return;
                }
            }
            PacketType::FloodRequest(mut _flood_request) => {
                // is it the first time the node receives this flood request?
                let current_flood = self.flood_initiators.get_key_value(&_flood_request.flood_id);
                let drone_flood = (&_flood_request.flood_id, &_flood_request.initiator_id);
                if current_flood.is_none() || current_flood.unwrap() != drone_flood{
                    // yes: send a flood request to all neighbors and add it to the flood_initiators hashmap
                    self.flood_initiators.insert(_flood_request.flood_id, _flood_request.initiator_id);
                    let p = self.forward_flood_request(packet, _flood_request);
                    match p{
                        Ok(_p) => {self.send_sent_to_sc(_p)}
                        Err(_p) => {panic!("*surprised quack*")}
                    }
                    return;
                } else {
                    // no: send a flood response
                    // add node to the path trace
                    _flood_request.increment(self.id, NodeType::Drone);
                    // generate a flood response
                    let flood_response_packet = _flood_request.generate_response(packet.session_id);
                    debug!("Drone: {:?} is generating a flood_request: {:?}", self.id, flood_response_packet);
                    let p = self.forward_packet(flood_response_packet);
                    match p{
                        Ok(_p) => {self.send_sent_to_sc(_p)}
                        Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                    }
                    return;
                }
            },
            PacketType::FloodResponse(_) => {
                let p = self.forward_packet(packet);
                match p{
                    Ok(_p) => {self.send_sent_to_sc(_p)}
                    Err(_p) => {self.send_shortcut_to_sc(_p.0)}
                }
                return;
            },
        }
    }
    fn forward_flood_request(&mut self, mut packet: Packet, mut flood_request: FloodRequest) ->Result<(Packet), SendError<Packet>>{
        packet.routing_header.increase_hop_index();

        // add node to the hops
        packet.routing_header.append_hop(self.id);

        // add node to the path trace
        flood_request.increment(self.id, NodeType::Drone);

        // generate new packet
        let p = Packet::new_flood_request(
            packet.routing_header.clone(),
            packet.session_id,
            flood_request,
        );

        // send packet to neighbors (except for the previous drone)
        let prev_node_id = packet.routing_header.previous_hop().expect("*surprised quack*");
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
    fn forward_packet(&mut self, mut packet: Packet) ->Result<(Packet), SendError<Packet>>{
        packet.routing_header.increase_hop_index();
        
        // Try to send packet
        match packet.routing_header.current_hop() {
            None => {panic!("*surprised quack*, Drone: {:?} pack: {:?}", self.id, packet)}
            Some(_next_node_id) => {self.try_send_packet(packet, _next_node_id)}
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
    // </editor-fold>
}