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
use crate::server::server::ServerEvent;

pub struct CommunicationServer1 {
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

pub trait ServerTrait {
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
}

impl ServerTrait for CommunicationServer1 {
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
        debug!("Server: {:?} started and waiting for packets", self.id);
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
}

impl CommunicationServer1 {
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
            // ClientServerCommand::RegistrationRequest(node_id) => {
            //     // Handle registration request
            // },
            // ClientServerCommand::RequestServerList(node_id) => {
            //     // Handle server list request
            // },
            // ClientServerCommand::RequestFileList(node_id) => {
            //     // Handle file list request
            // },
            ClientServerCommand::SendChatMessage(target_id, msg_id, msg) => {
                debug!("Server: {:?} sending chat message to {:?}: {:?}", self.id, target_id, msg);
            },
            // ClientServerCommand::RequestServerType(_) => {
            //     debug!("Server: {:?} received RequestServerType command", self.id);
            // },
            // ClientServerCommand::ResponseServerType(_) => {
            //     debug!("Server: {:?} received ResponseServerType command", self.id);
            // },
            ClientServerCommand::StartFloodRequest => {
                debug!("Server: {:?} received StartFloodRequest command", self.id);
            },
        }
    }
    fn handle_packet(&mut self, mut packet: Packet) {
        match &packet.pack_type {
            PacketType::MsgFragment(_fragment) => {
                debug!("Server: {:?} received a MsgFragment {:?}", self.id, _fragment);
            },
            PacketType::FloodResponse(_flood_response) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _flood_response);
            },
            PacketType::Ack(_ack) => {
                debug!("Server: {:?} received a Ack {:?}", self.id, _ack);
            },
            PacketType::Nack(_nack) => {
                debug!("Server: {:?} received a Ack {:?}", self.id, _nack);
            },
            PacketType::FloodRequest(_flood_request) => {
                debug!("Server: {:?} received a FloodRequest {:?}", self.id, _flood_request);
            },
        }
    }
}