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
use wg_2024::packet::PacketType::MsgFragment;

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

// nack acknowledgement, floodResponse (non si perdono)
impl MyDrone {
    fn handle_packet(&mut self, mut packet: Packet) {
        // step 1-2
        if self.id == packet.routing_header.hops[packet.routing_header.hop_index] {
            // check if it's a flood response, if it's the hod index should go backwards
            match packet.pack_type {
                PacketType::FloodResponse(_) => {
                    packet.routing_header.hop_index -= 1;
                },
                (_) => packet.routing_header.hop_index += {1}
            };
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
                session_id: packet.session_id,
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
                session_id: packet.session_id,
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
                session_id: packet.session_id,
            };
            // Todo: send it and terminate
            return;
        }

        // step 5
        match packet.pack_type {
            PacketType::Nack(_nack) => {
                let p = Packet {
                    pack_type: PacketType::Nack(Nack {
                        fragment_index: _nack.fragment_index,
                        nack_type: _nack.nack_type
                    }),
                    routing_header: SourceRoutingHeader {
                        hop_index: packet.routing_header.hop_index,
                        hops: packet.routing_header.hops
                    },
                    session_id: packet.session_id,
                };
                // Todo: send it and terminate
                return
            },
            PacketType::Ack(_ack) => {
                let p = Packet {
                    pack_type: PacketType::Ack(Ack {
                        fragment_index: _ack.fragment_index,
                    }),
                    routing_header: SourceRoutingHeader {
                        hop_index: packet.routing_header.hop_index,
                        hops: packet.routing_header.hops
                    },
                    session_id: packet.session_id,
                };
                // Todo: send it and terminate
                return
            }
            PacketType::MsgFragment(_fragment) => {
                let mut rng = rand::thread_rng();
                if rng.gen_range(0.0..=1.0) < self.pdr{
                    // todo() drop
                }
                let p = Packet {
                    pack_type: PacketType::MsgFragment(Fragment{
                        fragment_index: _fragment.fragment_index,
                        total_n_fragments: _fragment.total_n_fragments,
                        length: _fragment.length,
                        data: _fragment.data,
                    }),
                    routing_header: SourceRoutingHeader {
                        hop_index: packet.routing_header.hop_index,
                        hops: packet.routing_header.hops
                    },
                    session_id: packet.session_id,
                };
                // Todo: send it and terminate
                return
            }
            PacketType::FloodRequest(_flood_request) => {
                // check if it's the first it gets this flood request
                if _flood_request.path_trace.contains(&(self.id, NodeType::Drone)){
                    let p = Packet {
                        pack_type: PacketType::FloodResponse(FloodResponse{
                            flood_id: _flood_request.flood_id,
                            path_trace: _flood_request.path_trace,
                        }),
                        routing_header: SourceRoutingHeader {
                            hop_index: packet.routing_header.hop_index-2, // without the -2 it will try to go to the next drone before going backwards
                            hops: packet.routing_header.hops
                        },
                        session_id: packet.session_id,
                    };
                    // Todo: send it backwards and terminate
                    return
                } else {
                    let mut new_path_trace = _flood_request.path_trace;
                    new_path_trace.push((self.id, NodeType::Drone));
                    let p = Packet {
                        pack_type: PacketType::FloodRequest(FloodRequest{
                            flood_id: _flood_request.flood_id,
                            initiator_id: _flood_request.initiator_id,
                            path_trace: new_path_trace,
                        }),
                        routing_header: SourceRoutingHeader {
                            hop_index: packet.routing_header.hop_index,
                            hops: packet.routing_header.hops
                        },
                        session_id: packet.session_id,
                    };
                    // Todo: send it to drone neighbors (except the one from witch it received the packet) and terminate
                    return
                }
            },
            PacketType::FloodResponse(_flood_response) => {
                let p = Packet {
                    pack_type: PacketType::FloodResponse(FloodResponse{
                        flood_id: _flood_response.flood_id,
                        path_trace: _flood_response.path_trace,
                    }),
                    routing_header: SourceRoutingHeader {
                        hop_index: packet.routing_header.hop_index,
                        hops: packet.routing_header.hops
                    },
                    session_id: packet.session_id,
                };
                // Todo: keep sending it backwards and terminate
                return
            },
        }
    }
    fn send_packet(&mut self, packet: Packet, senders: HashMap<NodeId, Sender<Packet>>){
        for s in senders.iter(){
            s.1.send(packet.clone()).unwrap()
        }
    }
    
    fn handle_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::AddSender(_node_id, _sender) => {self.add_sender(_node_id, _sender)},
            DroneCommand::SetPacketDropRate(_pdr) =>{self.pdr = _pdr},
            DroneCommand::Crash => unreachable!(),
            _ => {}
        }
    }
    fn add_sender(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
}
