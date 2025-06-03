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
use crate::client::client::{Client, ClientEvent};
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
    MessageSent {
        target: NodeId,
        content: MessageContent,
    },
    MessageReceived {
        content: MessageContent,
    },
}

pub trait Server {
    type RequestType: Request;
    type ResponseType: Response;

    fn run(&mut self);
    fn send_packet_sent_to_sc(&mut self, packet: Packet){
        self.controller_send().send(ServerEvent::PacketSent(packet)).expect("this is fine 🔥☕");
    }
    fn send_packet_received_to_sc(&mut self, packet: Packet){
        self.controller_send().send(ServerEvent::PacketReceived(packet)).expect("this is fine 🔥☕");
    }
    fn send_message_sent_to_sc(&mut self, message: MessageContent, target: NodeId){
        self.controller_send().send(ServerEvent::MessageSent {target: target, content: message}).expect("this is fine 🔥☕");
    }
    fn send_message_received_to_sc(&mut self, message: MessageContent){
        self.controller_send().send(ServerEvent::MessageReceived { content: message }).expect("this is fine 🔥☕");
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