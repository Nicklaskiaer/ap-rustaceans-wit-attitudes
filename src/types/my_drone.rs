#![allow(unused)]

use crossbeam_channel::{select_biased, unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::{fs, thread};
use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, NodeEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{NackType, Packet, PacketType};
use wg_2024::drone::DroneOptions;
use wg_2024::packet::Nack;

struct MyDrone {
    id: NodeId,
    controller_send: Sender<NodeEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
}

impl Drone for MyDrone {
    fn new(options: DroneOptions) -> Self {
        Self {
            id: options.id,
            controller_send: options.controller_send,
            controller_recv: options.controller_recv,
            packet_recv: options.packet_recv,
            pdr: options.pdr,
            packet_send: HashMap::new(),
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

// nack acknowledgement, floodResponse (non si perdono)
impl MyDrone {
    fn handle_packet(&mut self, mut packet: Packet) {
        // step 1-2
        if self.id == packet.routing_header.hops[packet.routing_header.hop_index] {
            packet.routing_header.hop_index += 1;
        } else {
            Packet{
                pack_type: PacketType::Nack(Nack { 
                    fragment_index: match packet.pack_type {
                        PacketType::MsgFragment(_fragment) => {_fragment.fragment_index}
                        _ => 0,
                    }, 
                    nack_type: NackType::UnexpectedRecipient(self.id) 
                }),
                routing_header: SourceRoutingHeader { 
                    hop_index: packet.routing_header.hop_index, 
                    hops: packet.routing_header.hops 
                },
                session_id: 0,
            };
            // Todo: send it and terminate
            return;
        };
        
        // step 3
        if packet.routing_header.hops.len() == packet.routing_header.hop_index {
            Packet {
                pack_type: PacketType::Nack(Nack {
                    fragment_index: match packet.pack_type {
                        PacketType::MsgFragment(_fragment) => { _fragment.fragment_index }
                        _ => 0,
                    },
                    nack_type: NackType::DestinationIsDrone
                }),
                routing_header: SourceRoutingHeader {
                    hop_index: packet.routing_header.hop_index,
                    hops: packet.routing_header.hops
                },
                session_id: 0,
            };
            // Todo: send it and terminate
            return;
        }
        
        // step 4
        if !self.packet_send.contains_key(&packet.routing_header.hops[packet.routing_header.hop_index + 1]) {
            Packet {
                pack_type: PacketType::Nack(Nack {
                    fragment_index: match packet.pack_type {
                        PacketType::MsgFragment(_fragment) => { _fragment.fragment_index }
                        _ => 0,
                    },
                    nack_type: NackType::ErrorInRouting(packet.routing_header.hops[packet.routing_header.hop_index + 1])
                }),
                routing_header: SourceRoutingHeader {
                    hop_index: packet.routing_header.hop_index,
                    hops: packet.routing_header.hops
                },
                session_id: 0,
            };
            // Todo: send it and terminate
            return;
        }
        
        // step 5
        match packet.pack_type {
            PacketType::Nack(_nack) => todo!(),
            PacketType::Ack(_ack) => todo!(),
            PacketType::MsgFragment(_fragment) => todo!() //also check drop rate,
            PacketType::FloodRequest(_flood_request) => todo!(),
            PacketType::FloodResponse(_flood_response) => todo!(),
        }
    }
    fn handle_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::AddSender(_node_id, _sender) => todo!(),
            DroneCommand::SetPacketDropRate(_pdr) => todo!(),
            DroneCommand::Crash => unreachable!(),
        }
    }
    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
}
