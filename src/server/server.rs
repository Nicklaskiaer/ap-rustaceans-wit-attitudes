#[cfg(feature = "debug")]
use crate::debug;

use crate::assembler::assembler::*;
use crate::server::message::*;
use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use rand::random;
use serde::{Deserialize, Serialize};
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet;
use wg_2024::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType,
};
use crate::client::client::Client;
use crate::client::client_server_command::{compute_path_to_node, ClientServerCommand};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerType {
    ContentServer(ContentType),
    CommunicationServer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Media,
}

pub enum ServerEvent {
    PacketSent(Packet),
    PacketReceived(Packet),
}

pub trait Server {
    type RequestType: Request;
    type ResponseType: Response;

    fn run(&mut self);
    fn send_sent_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>> {
        self.controller_send().send(ServerEvent::PacketSent(packet))
    }
    fn send_recv_to_sc(&mut self, packet: Packet) -> Result<(), SendError<ServerEvent>> {
        self.controller_send().send(ServerEvent::PacketReceived(packet))
    }

    fn controller_recv(&self) -> &Receiver<ClientServerCommand>;
    fn packet_recv(&self) -> &Receiver<Packet>;
    fn assembler_recv(&self) -> &Receiver<Vec<u8>>;
    fn controller_send(&mut self) -> &mut Sender<ServerEvent>;
    fn id(&self) -> NodeId;
    fn packet_send(&self) -> &HashMap<NodeId, Sender<Packet>>;
    fn topology_map(&self) -> &HashSet<(NodeId, Vec<NodeId>)>;
    fn topology_map_mut(&mut self) -> &mut HashSet<(NodeId, Vec<NodeId>)>;
    fn assemblers_mut(&mut self) -> &mut Vec<Assembler>;
}