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
use crate::client::client_server_command::{send_fragment_to_assembler, try_send_packet, try_send_packet_with_target_id, update_topology_with_flood_response, ClientServerCommand};
use crate::server::server::{Server, ServerEvent};

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

impl Server for CommunicationServer {
    type RequestType = ChatRequest;
    type ResponseType = ChatResponse;

    fn run(&mut self) {
        debug!("Communication Server: {:?} started and waiting for packets", self.id);
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
                        self.handle_assembler_data(data);
                    }
                },
            }
        }
    }
    fn controller_recv(&self) -> &Receiver<ClientServerCommand> {
        &self.controller_recv
    }
    fn packet_recv(&self) -> &Receiver<Packet> {
        &self.packet_recv
    }
    fn assembler_recv(&self) -> &Receiver<Vec<u8>> {
        &self.assembler_recv
    }
    fn controller_send(&mut self) -> &mut Sender<ServerEvent> {
        &mut self.controller_send
    }
    fn id(&self) -> NodeId {
        self.id
    }
    fn packet_send(&self) -> &HashMap<NodeId, Sender<Packet>> {
        &self.packet_send
    }
    fn topology_map(&self) -> &HashSet<(NodeId, Vec<NodeId>)> {
        &self.topology_map
    }
    fn topology_map_mut(&mut self) -> &mut HashSet<(NodeId, Vec<NodeId>)> {
        &mut self.topology_map
    }
    fn assemblers_mut(&mut self) -> &mut Vec<Assembler> {
        &mut self.assemblers
    }
}

impl CommunicationServer {
    pub fn new(
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
            ClientServerCommand::SendChatMessage(target_id, id, msg) => {
                debug!("Server: {:?} sending chat message to {:?}: {:?}", self.id, target_id, msg);
            },
            ClientServerCommand::StartFloodRequest => {
                debug!("Server: {:?} received StartFloodRequest command", self.id);

                // Generate a unique flood ID using current time
                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                let flood_id = timestamp ^ random::<u64>();

                // Create path trace with just this server
                let path_trace = vec![(self.id, NodeType::Server)];

                // Send flood request to all connected drones
                for drone_id in &self.connected_drone_ids {
                    let flood_request = Packet::new_flood_request(
                        SourceRoutingHeader {
                            hop_index: 1,
                            hops: vec![self.id, *drone_id],
                        },
                        flood_id,
                        FloodRequest {
                            flood_id,
                            initiator_id: self.id,
                            path_trace: path_trace.clone(),
                        },
                    );

                    // Try to send packet
                    try_send_packet_with_target_id(self.id, drone_id, &flood_request, &self.packet_send);
                }
            },
            ClientServerCommand::RequestServerType => {/* servers do not need to use it */},
        }
    }
    fn handle_packet(&mut self, mut packet: Packet) {
        match &packet.pack_type {
            PacketType::Nack(_nack) => {
                debug!("Server: {:?} received a Ack {:?}", self.id, _nack);
            },
            PacketType::Ack(_ack) => {
                debug!("Server: {:?} received a Ack {:?}", self.id, _ack);
            },
            PacketType::MsgFragment(_fragment) => {
                debug!("Server: {:?} received a MsgFragment {:?}", self.id, _fragment);

                // Send fragment to assembler to be reassembled
                match send_fragment_to_assembler(packet.clone(), &mut self.assemblers) {
                    Ok(_) => {
                        debug!("Server: {:?} sent fragment to assembler", self.id);

                        // Send ack back to the sender
                        let mut ack_packet = Packet::new_ack(
                            packet.routing_header.get_reversed(),
                            packet.session_id,
                            _fragment.fragment_index
                        );

                        // Try to send packet
                        ack_packet.routing_header.increase_hop_index();
                        try_send_packet(self.id, &ack_packet, &self.packet_send);
                    },
                    Err(e) => {
                        debug!("ERROR: Server {:?} failed to send fragment to assembler: {}", self.id, e);
                    }
                }
            },
            PacketType::FloodRequest(_flood_request) => {
                debug!("Server: {:?} received a FloodRequest {:?}", self.id, _flood_request);

                // send a flood response
                // add node to the path trace
                let mut flood_request = _flood_request.clone();
                flood_request.increment(self.id, NodeType::Server);
                // generate a flood response
                let mut flood_response_packet = flood_request.generate_response(packet.session_id);
                debug!("Server: {:?} is generating a FloodResponse: {:?}", self.id, flood_response_packet);
                flood_response_packet.routing_header.increase_hop_index();

                // Try to send packet
                try_send_packet(self.id, &flood_response_packet, &self.packet_send);
            },
            PacketType::FloodResponse(_flood_response) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _flood_response);
                update_topology_with_flood_response(self.id, _flood_response, &mut self.topology_map);
            },
        }
    }
    fn handle_assembler_data(&mut self, mut data: Vec<u8>) {
        if let Ok(str_data) = String::from_utf8(data.clone()) {
            debug!("Server {:?} received assembled message: {:?}", self.id, str_data);
        }
    }
}

