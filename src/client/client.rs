#[cfg(feature = "debug")]
use crate::debug;

use crossbeam_channel::{after, select_biased, unbounded, Receiver, SendError, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use std::thread;
use std::thread::ThreadId;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet;
use wg_2024::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType,
};
use rand::{Rng, thread_rng, random};
use crate::assembler::assembler::Assembler;
use crate::client::client_server_command::{compute_path_to_node, send_fragment_to_assembler, send_message_in_fragments, try_send_packet, try_send_packet_with_target_id, update_topology_with_flood_response, ClientServerCommand};
use crate::server::message::{DroneSend, Message, MessageContent, ServerTypeRequest, ServerTypeResponse, TextRequest, TextResponse};
use crate::server::server::{ServerEvent, ServerType};

pub struct Client {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ClientEvent>,
    controller_send_itself: Sender<ClientServerCommand>, // sender to itself (used for delayed commands)
    controller_recv: Receiver<ClientServerCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    assemblers: Vec<Assembler>,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    server_type_map: HashMap<NodeId, Option<ServerType>>,
    assembler_send: Sender<Vec<u8>>,
    assembler_recv: Receiver<Vec<u8>>,
}

pub enum ClientEvent {
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

pub trait ClientTrait {
    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ClientEvent>,
        controller_send_itself: Sender<ClientServerCommand>,
        controller_recv: Receiver<ClientServerCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        assemblers: Vec<Assembler>,
        topology_map: HashSet<(NodeId, Vec<NodeId>)>,
        server_type_map: HashMap<NodeId, Option<ServerType>>,
        assembler_send: Sender<Vec<u8>>,
        assembler_recv: Receiver<Vec<u8>>,
    ) -> Self;

    fn run(&mut self);
}

impl ClientTrait for Client {
    fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ClientEvent>,
        controller_send_itself: Sender<ClientServerCommand>,
        controller_recv: Receiver<ClientServerCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        assemblers: Vec<Assembler>,
        topology_map: HashSet<(NodeId, Vec<NodeId>)>,
        server_type_map: HashMap<NodeId, Option<ServerType>>,
        assembler_send: Sender<Vec<u8>>,
        assembler_recv: Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            id,
            connected_drone_ids,
            controller_send,
            controller_send_itself,
            controller_recv,
            packet_recv,
            packet_send,
            assemblers,
            topology_map,
            server_type_map,
            assembler_send,
            assembler_recv,
        }
    }

    fn run(&mut self) {
        debug!("Client: {:?} started and waiting for packets", self.id);
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
}

impl Client {
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
            ClientServerCommand::SendChatMessage(node_id, id, msg) => {
                debug!("Client: {:?} sending chat message to {:?}: {}", self.id, node_id, msg);
            },
            ClientServerCommand::StartFloodRequest => {
                debug!("Client: {:?} received StartFloodRequest command", self.id);

                // Generate a unique flood ID using current time
                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                let flood_id = timestamp ^ random::<u64>();
                
                // Create path trace with just this client
                let path_trace = vec![(self.id, NodeType::Client)];

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

                // Spawn thread to wait and then send RequestServerType
                let controller_send_itself = self.controller_send_itself.clone();
                thread::spawn(move || {
                    thread::sleep(std::time::Duration::from_secs(3));
                    controller_send_itself.send(ClientServerCommand::RequestServerType).ok();
                });
            },
            ClientServerCommand::RequestServerType => {
                debug!("Client: {:?} received RequestServerType command, servers found: {:?}", self.id, self.server_type_map);

                // Query all servers in the server_type_map that have None as their type
                for server_id in self.server_type_map.keys().cloned().collect::<Vec<_>>() {
                    if let Some(None) = self.server_type_map.get(&server_id) {
                        self.send_server_type_request(server_id);
/*                        
                        // TODO: remove it
                        // test 11->42
                        if self.id == 11 && server_id == 42 {
                            self.send_server_type_request(server_id);
                        }*/
                    }
                }
            },
            ClientServerCommand::RequestFileList(node_id) => {
                debug!("Client: {:?} received RequestFileList, Server id: {:?}", self.id, node_id);
                self.send_text_request_TextList(node_id, ); 
            },
            ClientServerCommand::RequestFile(node_id, file_id) => {
                debug!("Client: {:?} received RequestFile, Server id: {:?} file id: {:?}", self.id, node_id, file_id);
                self.send_text_request_Text(node_id, file_id); 
            },
        }
    }
    fn handle_packet(&mut self, mut packet: Packet) {
        self.send_packet_received_to_sc(packet.clone());
        match &packet.pack_type {
            PacketType::Nack(_nack) => {
                debug!("Client: {:?} received a Nack {:?}", self.id, _nack);
            },
            PacketType::Ack(_ack) => {
                debug!("Client: {:?} received a Ack {:?}", self.id, _ack);
            },
            PacketType::MsgFragment(_fragment) => {
                debug!("Client: {:?} received a MsgFragment {:?}", self.id, _fragment);

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
                debug!("Client: {:?} received a FloodRequest {:?}", self.id, _flood_request);

                // send a flood response
                // add node to the path trace
                let mut flood_request = _flood_request.clone();
                flood_request.increment(self.id, NodeType::Client);
                // generate a flood response
                let mut flood_response_packet = flood_request.generate_response(packet.session_id);
                debug!("Server: {:?} is generating a FloodResponse: {:?}", self.id, flood_response_packet);
                flood_response_packet.routing_header.increase_hop_index();

                // Try to send packet
                try_send_packet(self.id, &flood_response_packet, &self.packet_send);
            },
            PacketType::FloodResponse(_flood_response) => {
                debug!("Client: {:?} received a FloodResponse {:?}", self.id, _flood_response);
                update_topology_with_flood_response(self.id, _flood_response, &mut self.topology_map);

                // if it's a server add it to the server_type_map
                let &(node_id, _node_type) = _flood_response.path_trace.last().unwrap();
                // TODO: change None not after the flood but after that server is called for the first time
                if _node_type == NodeType::Server {
                    self.server_type_map.insert(node_id, None);
                }
            },
        }
    }
    fn handle_assembler_data(&mut self, mut data: Vec<u8>) {
        if let Ok(str_data) = String::from_utf8(data.clone()) {
            debug!("Client {:?} received assembled message: {:?}", self.id, str_data);

            // Try to parse as ServerTypeResponse
            if let Ok(message) = serde_json::from_str::<Message<ServerTypeResponse>>(&str_data) {
                // Send to SC
                let content = MessageContent::ServerTypeResponse(message.content.clone());
                self.send_message_received_to_sc(content);
                
                match &message.content {
                    ServerTypeResponse::ServerType(server_type) => {
                        debug!("Client: {:?} received server type {:?} from {:?}", self.id, server_type, message.source_id);
                        self.server_type_map.insert(message.source_id, Some(server_type.clone()));
                    }
                }
            }
            // Try to parse as TextResponse
            else if let Ok(message) = serde_json::from_str::<Message<TextResponse>>(&str_data) {
                // Send to SC
                let content = MessageContent::TextResponse(message.content.clone());
                self.send_message_received_to_sc(content);

                match message.content {
                    TextResponse::TextList(file_list) => {
                        debug!("Client: {:?} received TextResponse::TextList from {:?} file list: {:?}", self.id, message.source_id, file_list);
                    },
                    TextResponse::Text(file) => {
                        debug!("Client: {:?} received TTextResponse::Text from {:?} file: {:?}", self.id, message.source_id, file);
                    },
                    TextResponse::NotFound => {
                        debug!("Client: {:?} received TextResponse::NotFound from {:?}", self.id, message.source_id);
                    },
                }
            }
            }
        }
    fn send_packet_sent_to_sc(&mut self, packet: Packet){
        self.controller_send.send(ClientEvent::PacketSent(packet)).expect("this is fine 🔥☕");
    }
    fn send_packet_received_to_sc(&mut self, packet: Packet){
        self.controller_send.send(ClientEvent::PacketReceived(packet)).expect("this is fine 🔥☕");
    }
    fn send_message_sent_to_sc(&mut self, message: MessageContent, target: NodeId){
        self.controller_send.send(ClientEvent::MessageSent {target: target, content: message}).expect("this is fine 🔥☕");
    }
    fn send_message_received_to_sc(&mut self, message: MessageContent){
        self.controller_send.send(ClientEvent::MessageReceived { content: message }).expect("this is fine 🔥☕");
    }
    fn send_server_type_request(&mut self, server_id: NodeId) {
        // Create a server type request with random session ID
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: ServerTypeRequest::GetServerType,
        };
        debug!("Server: {:?} sending msg to client {:?}, msg: {:?}", self.id, server_id, message);
        send_message_in_fragments(self.id, server_id, session_id, message, &self.packet_send, &self.topology_map);
    }
    fn send_text_request_TextList(&mut self, server_id: NodeId) {
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: TextRequest::TextList,
        };
        debug!("Server: {:?} sending msg to client {:?}, msg: {:?}", self.id, server_id, message);
        send_message_in_fragments(self.id, server_id, session_id, message, &self.packet_send, &self.topology_map);
    }
    fn send_text_request_Text(&mut self, server_id: NodeId, file_id: u64) {
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: TextRequest::Text(file_id),
        };
        debug!("Server: {:?} sending msg to client {:?}, msg: {:?}", self.id, server_id, message);
        send_message_in_fragments(self.id, server_id, session_id, message, &self.packet_send, &self.topology_map);
    }
}

