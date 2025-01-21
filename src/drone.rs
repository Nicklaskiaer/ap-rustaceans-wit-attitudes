use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use rand::{Rng};

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
    packet_send: HashMap<NodeId, Sender<Packet>>,
    pdr: f32,
    flood_cache: HashSet<(NodeId, u64)>,
    should_send_dropped: bool
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
            flood_cache: HashSet::new(),
            should_send_dropped: false
        }
    }

    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
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
            DroneCommand::SetPacketDropRate(_pdr) =>{
                self.pdr = _pdr
            },
            DroneCommand::Crash => {
                self.crash()
            },
            DroneCommand::AddSender(_node_id, _sender) => {
                self.packet_send.insert(_node_id, _sender);
            },
            DroneCommand::RemoveSender(_node_id) => {
                self.packet_send.remove(&_node_id);
            },
        }
    }
    fn crash(&mut self){
        // While in this loop the drone is in crashing state
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        match command {
                            // If no senders are left, the drone can exit the crashing state and be considered as crashed
                            DroneCommand::RemoveSender(_node_id) => {
                                self.packet_send.remove(&_node_id);
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
                                let hops = packet.routing_header.hops.clone();
                                let hop_index = packet.routing_header.hop_index+1;
                                self.send_ack(packet, _ack, SourceRoutingHeader{ hop_index, hops })
                            }
                            PacketType::Nack(_nack) => {
                                let hops = packet.routing_header.hops.clone();
                                let hop_index = packet.routing_header.hop_index+1;
                                self.send_nack(packet, _nack.nack_type, SourceRoutingHeader{ hop_index, hops })
                            }
                            PacketType::FloodResponse(_flood_response) => {
                                let hops = packet.routing_header.hops.clone();
                                let hop_index = packet.routing_header.hop_index+1;
                                self.send_flood_response(packet, _flood_response, SourceRoutingHeader{ hop_index, hops })
                            }

                            // Send Nack(ErrorInRouting) for other packet types
                            PacketType::MsgFragment(_) => {
                                let hops = packet.routing_header.hops.clone();
                                let hop_index = packet.routing_header.hop_index+1;
                                self.send_nack(
                                    packet,
                                    NackType::ErrorInRouting(self.id), SourceRoutingHeader{ hop_index, hops }
                                )
                            }
                        }
                    }
                }
            }
        }
    }
    // </editor-fold>

    // <editor-fold desc="Packets">
    fn handle_packet(&mut self, packet: Packet) {
        println!("{:?}, {:?}",self.id, self.packet_send);
        
        // handle flood request before anything (they are false positive for UnexpectedRecipient, DestinationIsDrone and ErrorInRouting)
        if let PacketType::FloodRequest(mut _flood_request) = packet.pack_type.clone(){
            println!("{:?}, {:?}",self.id, _flood_request.flood_id);
            
            // is it the first time the node receives this flood request?
            let is_in_cache = self.flood_cache.contains(&(self.id, _flood_request.flood_id));
            
            // does it have neighbors (except for the previous drone)?
            let has_neighbors = self.packet_send.len()>1;
            
            if !is_in_cache && has_neighbors {
                self.flood_cache.insert((self.id, _flood_request.flood_id)); // add to the cache

                // should add the node to the hops?
                let mut new_hops = packet.routing_header.hops.clone();
                if !new_hops.contains(&self.id){
                    new_hops.push(self.id);
                }

                // add node to the path trace
                _flood_request.path_trace.push((self.id, NodeType::Drone)); 

                // generate new packet
                let p = Packet {
                    pack_type: PacketType::FloodRequest(_flood_request),
                    routing_header: SourceRoutingHeader {
                        hop_index: packet.routing_header.hop_index,
                        hops: new_hops,
                    },
                    session_id: packet.session_id,
                };

                // send packet to neighbors (except for the previous drone)
                let prev_node_id = p.routing_header.previous_hop().unwrap()-1;
                for (node_id, _) in self.packet_send.clone(){
                    println!("{:?} is sending to {:?}. prev_node_id: {:?}", self.id, node_id, prev_node_id);
                    if prev_node_id != node_id{
                        // try to send packet
                        self.try_send_packet(p.clone(), node_id)
                    }
                }
                return;
            } else {
                if !is_in_cache{
                    self.flood_cache.insert((self.id, _flood_request.flood_id)); // add to the cache
                }

                // should add the node to the hops?
                let mut reversed_srh = packet.routing_header.clone();
                if !reversed_srh.hops.contains(&self.id){
                    reversed_srh.append_hop(self.id);
                }
                
                reversed_srh.reverse();

                // should add the node to the path trace?
                let mut new_path_trace = _flood_request.path_trace.clone();
                if !new_path_trace.contains(&(self.id, NodeType::Drone)){
                    new_path_trace.push((self.id, NodeType::Drone));
                }

                self.send_flood_response(
                    packet,
                    FloodResponse{
                        flood_id: _flood_request.flood_id,
                        path_trace: new_path_trace,
                    },
                    SourceRoutingHeader{ hop_index: reversed_srh.hop_index+1, hops: reversed_srh.hops }
                );
                return;
            }
        }

        // check for UnexpectedRecipient (current drone is not the intended receiver)
        if packet.routing_header.current_hop().unwrap() != self.id{
            let mut reversed_srh = packet.routing_header.clone();
            reversed_srh = reversed_srh.sub_route(0..reversed_srh.hop_index+1).unwrap();
            reversed_srh.reverse();
            reversed_srh.reset_hop_index();

            // change the wrong hop id with this drone id 
            // reversed_srh.hops[0] = self.id;
            reversed_srh.increase_hop_index();

            self.send_nack(
                packet,
                NackType::UnexpectedRecipient(self.id),
                reversed_srh
            );
            return;
        }

        // check for DestinationIsDrone (current drone is the last hop)
        if packet.routing_header.is_last_hop(){
            let mut reversed_srh = packet.routing_header.clone();
            reversed_srh = reversed_srh.sub_route(0..reversed_srh.hop_index+1).unwrap();
            reversed_srh.reverse();
            reversed_srh.reset_hop_index();
            reversed_srh.increase_hop_index();

            self.send_nack(
                packet,
                NackType::DestinationIsDrone,
                reversed_srh
            );
            return;
        }

        // check for ErrorInRouting (next hop is not a neighbor of the drone)
        if !self.packet_send.contains_key(&packet.routing_header.next_hop().unwrap()){
            let mut reversed_srh = packet.routing_header.clone();
            reversed_srh = reversed_srh.sub_route(0..reversed_srh.hop_index+1).unwrap();
            reversed_srh.reverse();
            reversed_srh.reset_hop_index();
            reversed_srh.increase_hop_index();

            self.send_nack(
                packet.clone(),
                NackType::ErrorInRouting(packet.routing_header.hops[&packet.routing_header.hop_index+1]),
                reversed_srh
            );
            return;
        }
        
        // increase the hop index
        let hops = packet.routing_header.hops.clone();
        let hop_index = packet.routing_header.hop_index+1;

        // match with all Packet Types
        match packet.clone().pack_type {
            PacketType::Nack(_nack) => {
                self.send_nack(
                    packet,
                    _nack.nack_type,
                    SourceRoutingHeader{ hop_index, hops }
                );
                return;
            }
            PacketType::Ack(_ack) => {
                self.send_ack(
                    packet,
                    _ack,
                    SourceRoutingHeader{ hop_index, hops }
                );
                return;
            }
            PacketType::MsgFragment(_fragment) => {
                // check if it's Dropped
                let mut rng = rand::thread_rng();
                if rng.gen_range(0.0..=1.0) < self.pdr {
                    let mut reversed_srh = packet.routing_header.clone();
                    reversed_srh = reversed_srh.sub_route(0..reversed_srh.hop_index+1).unwrap();
                    reversed_srh.reverse();
                    reversed_srh.reset_hop_index();
                    reversed_srh.increase_hop_index();
                    
                    self.should_send_dropped = true;
                    
                    self.send_nack(
                        packet.clone(),
                        NackType::Dropped,
                        reversed_srh
                    );
                    return;
                } else {
                    // send fragment
                    self.send_msg_fragment(
                        packet,
                        _fragment,
                        SourceRoutingHeader{ hop_index, hops }
                    );
                    return;
                }
            },
            PacketType::FloodResponse(_flood_response) => {
                self.send_flood_response(
                    packet,
                    _flood_response,
                    SourceRoutingHeader{ hop_index, hops }
                );
                return;
            },
            PacketType::FloodRequest(_) => {},
        }
    }
    fn send_nack(&mut self, packet: Packet, nack_type: NackType, source_routing_header: SourceRoutingHeader){
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
            routing_header: source_routing_header,
            session_id: packet.session_id,
        };

        // try to send packet
        let next_node_id = p.routing_header.hops[p.routing_header.hop_index];
        self.try_send_packet(p, next_node_id)
    }
    fn send_ack(&mut self, packet: Packet, ack: Ack, source_routing_header: SourceRoutingHeader){
        // generate new packet
        let p = Packet {
            pack_type: PacketType::Ack(Ack {
                fragment_index: ack.fragment_index,
            }),
            routing_header: source_routing_header,
            session_id: packet.session_id,
        };

        // try to send packet
        let next_node_id = p.routing_header.hops[p.routing_header.hop_index];
        self.try_send_packet(p, next_node_id)
    }
    fn send_msg_fragment(&mut self, packet: Packet, fragment: Fragment, source_routing_header: SourceRoutingHeader){
        // generate new packet
        let p = Packet {
            pack_type: PacketType::MsgFragment(Fragment{
                fragment_index: fragment.fragment_index,
                total_n_fragments: fragment.total_n_fragments,
                length: fragment.length,
                data: fragment.data,
            }),
            routing_header: source_routing_header,
            session_id: packet.session_id,
        };

        // try to send packet
        let next_node_id = p.routing_header.current_hop().unwrap();
        self.try_send_packet(p, next_node_id)
    }
    /*fn send_flood_request(&mut self, packet: Packet, flood_request: FloodRequest, source_routing_header: SourceRoutingHeader){
        // add node to the hops
        let mut new_hops = source_routing_header.hops.clone();
        // new_hops.push(self.id);

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
                hop_index: source_routing_header.hop_index-1,
                hops: new_hops,
            },
            session_id: packet.session_id,
        };

        // send packet to neighbors (except for the previous drone)
        let prev_node_id = p.routing_header.previous_hop().unwrap()-1;
        for (node_id, _) in self.packet_send.clone(){
            println!("{:?} is sending to {:?}. prev_node_id: {:?}", self.id, node_id, prev_node_id);
            if prev_node_id != node_id{
                // try to send packet
                self.try_send_packet(p.clone(), node_id)
            }
        }
    }*/
    fn send_flood_response(&mut self, packet: Packet, flood_response: FloodResponse, source_routing_header: SourceRoutingHeader){
        // generate new packet
        let p = Packet {
            pack_type: PacketType::FloodResponse(FloodResponse{
                flood_id: flood_response.flood_id,
                path_trace: flood_response.path_trace,
            }),
            routing_header: source_routing_header,
            session_id: packet.session_id,
        };
        
        // try to send packet
        let next_node_id = p.routing_header.hops[p.routing_header.hop_index];
        println!("{:?} is sending to {:?}", self.id, next_node_id);
        self.try_send_packet(p, next_node_id)
    }
    fn try_send_packet(&mut self, p: Packet, next_node_id: NodeId) {
        if let Some(sender) = self.packet_send.get(&next_node_id) {
            if sender.send(p.clone()).is_ok() {
                if self.should_send_dropped{
                    self.controller_send.send(DroneEvent::PacketDropped(p)).unwrap();
                    self.should_send_dropped = false;
                } else{
                    self.controller_send.send(DroneEvent::PacketSent(p)).unwrap();
                }
                // if !matches!(p.pack_type, PacketType::Nack(Nack { nack_type: NackType::Dropped, .. })) {
                //     self.controller_send.send(DroneEvent::PacketSent(p)).unwrap();
                // }
            } else {
                panic!("cannot send: {:?}", p);
            }
        } else {
            panic!("{:?} cannot send to {:?}, cannot send: {:?}\nDrone neighbors: {:?}", self.id, next_node_id, p, self.packet_send);
        }
    }
    // </editor-fold>
}
