use std::any::Any;
#[cfg(feature = "debug")]
use crate::debug;

use crossbeam_channel::{after, select_biased, unbounded, Receiver, SendError, Sender};
use std::collections::{HashMap, HashSet, VecDeque};
use std::thread;
use std::thread::ThreadId;
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet;
use wg_2024::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, NackType, NodeType, Packet, PacketType,
};
use rand::{Rng, thread_rng, random};
use serde::{Deserialize, Serialize};
use crate::assembler::assembler::Assembler;
use crate::client_server::network_core::{ClientEvent, ClientServerCommand, ContentType, NetworkNode, ServerType};
use crate::message::message::{ChatRequest, ChatResponse, MediaResponse, Message, MessageContent, ServerTypeRequest, ServerTypeResponse, TextRequest, TextResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerTypeWithSessionId {
    SessionId,
    ContentServer(ContentType),
    CommunicationServer,
}

pub struct Client {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ClientEvent>,
    controller_send_itself: Sender<ClientServerCommand>, // sender to itself (used for delayed commands)
    controller_recv: Receiver<ClientServerCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    server_type_map: HashMap<NodeId, Option<ServerType>>,
    session_ids_for_request_server_type: HashSet<u64>,
    assembler_send: Sender<Packet>,
    // assembler_recv: Receiver<Packet>,
    // assembler_res_send: Sender<Vec<u8>>,
    assembler_res_recv: Receiver<Vec<u8>>,
}

impl NetworkNode for Client {
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
    // fn assembler_send(&self) -> &Sender<Packet> {
    //     &self.assembler_send
    // }

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
                recv(self.assembler_res_recv) -> data => {
                    if let Ok(data) = data {
                        self.handle_assembler_data(data);
                    }
                },
            }
        }
    }
    fn send_packet_sent_to_sc(&mut self, packet: Packet) {
        self.controller_send
            .send(ClientEvent::PacketSent(packet))
            .expect("this is fine 🔥☕");
    }
    fn send_packet_received_to_sc(&mut self, packet: Packet) {
        self.controller_send
            .send(ClientEvent::PacketReceived(packet))
            .expect("this is fine 🔥☕");
    }
    fn send_message_sent_to_sc(&mut self, message: MessageContent, target: NodeId) {
        self.controller_send
            .send(ClientEvent::MessageSent {
                from: self.id,
                to: target,
                content: message,
            })
            .expect("this is fine 🔥☕");
    }
    fn send_message_received_to_sc(&mut self, message: MessageContent) {
        self.controller_send
            .send(ClientEvent::MessageReceived {
                receiver: self.id,
                content: message
                })
            .expect("this is fine 🔥☕");
    }
}

impl Client {
    pub fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ClientEvent>,
        controller_send_itself: Sender<ClientServerCommand>,
        controller_recv: Receiver<ClientServerCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        topology_map: HashSet<(NodeId, Vec<NodeId>)>,
        server_type_map: HashMap<NodeId, Option<ServerType>>,
        session_ids_for_request_server_type: HashSet<u64>,
        assembler_send: Sender<Packet>,
        // assembler_recv: Receiver<Packet>,
        // assembler_res_send: Sender<Vec<u8>>,
        assembler_res_recv: Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            id,
            connected_drone_ids,
            controller_send,
            controller_send_itself,
            controller_recv,
            packet_recv,
            packet_send,
            topology_map,
            server_type_map,
            session_ids_for_request_server_type,
            assembler_send,
            // assembler_recv,
            // assembler_res_send,
            assembler_res_recv,
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
                recv(self.assembler_res_recv) -> data => {
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
                    DroneCommand::SetPacketDropRate(_) => {}
                    DroneCommand::Crash => {}
                    DroneCommand::AddSender(_id, _sender) => {}
                    DroneCommand::RemoveSender(_id) => {}
                }
            },
            ClientServerCommand::SendChatMessage(node_id, msg) => {
                debug!("Client: {:?} received SendChatMessage command", self.id);

                self.send_chat_message(node_id, msg);
            },
            ClientServerCommand::StartFloodRequest => {
                debug!("Client: {:?} received StartFloodRequest command", self.id);

                // Generate a unique flood ID using current time
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let flood_id = timestamp ^ random::<u64>();

                // Create path trace with just this client
                let path_trace = vec![(self.id, NodeType::Client)];

                // Send flood request to all connected drones
                let drone_ids = self.connected_drone_ids.clone();
                for drone_id in &drone_ids {
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
                    self.try_send_packet_with_target_id(drone_id, &flood_request);
                }

                // Spawn thread to wait and then send RequestServerType
                let controller_send_itself = self.controller_send_itself.clone();
                thread::spawn(move || {
                    thread::sleep(std::time::Duration::from_secs(3));
                    controller_send_itself
                        .send(ClientServerCommand::RequestServerType)
                        .ok();
                });
            },
            ClientServerCommand::RequestServerType => {
                debug!(
                    "Client: {:?} received RequestServerType command, servers found: {:?}",
                    self.id, self.server_type_map
                );

                // Query all servers in the server_type_map that have None as their type
                for server_id in self.server_type_map.keys().cloned().collect::<Vec<_>>() {
                    if let Some(None) = self.server_type_map.get(&server_id) {
                        self.send_server_type_request(server_id);
                    }
                }
            },
            ClientServerCommand::RegistrationRequest(node_id) => {
                debug!("Client: {:?} received RegistrationRequest command", node_id);

                self.send_registration_request(node_id);
            },
            ClientServerCommand::RequestFileList(node_id) => {
                debug!(
                    "Client: {:?} received RequestFileList, Server id: {:?}",
                    self.id, node_id
                );
                self.send_text_request_text_list(node_id);
            }
            ClientServerCommand::RequestFile(node_id, file_id) => {
                debug!(
                    "Client: {:?} received RequestFile, Server id: {:?} file id: {:?}",
                    self.id, node_id, file_id
                );
                self.send_text_request_text(node_id, file_id);
            }
            ClientServerCommand::TestCommand => {
                debug!(
                    "\n\
                    \nClient: {:?}\
                    \ntopology_map: {:?}\
                    \nserver_type_map: {:?}\
                    \nsession_ids_for_request_server_type: {:?}\
                    \n",
                    self.id, self.topology_map, self.server_type_map, self.session_ids_for_request_server_type
                );
            }
        }
    }
    fn handle_packet(&mut self, packet: Packet) {
        self.send_packet_received_to_sc(packet.clone());

        match &packet.pack_type {
            PacketType::Nack(_nack) => {
                debug!("Client: {:?} received a Nack {:?}", self.id, _nack);

                // If a request server type was dropped, a new one will be created
                if self.session_ids_for_request_server_type.contains(&packet.session_id) {
                    if let Some(server_id) = packet.routing_header.destination() {
                        self.send_server_type_request(server_id);
                    }
                }
            },
            PacketType::Ack(_ack) => {
                debug!("Client: {:?} received a Ack {:?}", self.id, _ack);
            },
            PacketType::MsgFragment(_fragment) => {
                debug!(
                    "Client: {:?} received a MsgFragment {:?}",
                    self.id, _fragment
                );

                // Send fragment to assembler to be reassembled
                match self.send_fragment_to_assembler(packet.clone()) {
                    Ok(_) => {
                        debug!("Client: {:?} sent fragment to assembler", self.id);

                        // Send ack back to the sender
                        let mut ack_packet = Packet::new_ack(
                            packet.routing_header.get_reversed(),
                            packet.session_id,
                            _fragment.fragment_index,
                        );

                        // Try to send packet
                        ack_packet.routing_header.increase_hop_index();
                        self.try_send_packet(&ack_packet);
                    }
                    Err(_e) => {
                        debug!(
                            "ERROR: Server {:?} failed to send fragment to assembler: {}",
                            self.id, _e
                        );
                    }
                }
            },
            PacketType::FloodRequest(_flood_request) => {
                debug!(
                    "Client: {:?} received a FloodRequest {:?}",
                    self.id, _flood_request
                );

                // send a flood response
                // add node to the path trace
                let mut flood_request = _flood_request.clone();
                flood_request.increment(self.id, NodeType::Client);
                // generate a flood response
                let mut flood_response_packet = flood_request.generate_response(packet.session_id);
                debug!(
                    "Server: {:?} is generating a FloodResponse: {:?}",
                    self.id, flood_response_packet
                );
                flood_response_packet.routing_header.increase_hop_index();

                // Try to send packet
                self.try_send_packet(&flood_response_packet);
            },
            PacketType::FloodResponse(_flood_response) => {
                debug!(
                    "Client: {:?} received a FloodResponse {:?}",
                    self.id, _flood_response
                );
                self.update_topology_with_flood_response(_flood_response, true);

                // if it's a server add it to the server_type_map
                let &(node_id, _node_type) = _flood_response.path_trace.last().unwrap();
                if _node_type == NodeType::Server {
                    self.server_type_map.insert(node_id, None);
                }
            },
        }
    }
    fn handle_assembler_data(&mut self, data: Vec<u8>) {
        if let Ok(str_data_raw) = String::from_utf8(data.clone()) {
            debug!(
                "Client {:?} received assembled message: {:?}",
                self.id, str_data_raw
            );

            if let Some(str_data) = str_data_raw.split("\0").nth(0) {
                // Try to parse as ServerTypeResponse
                if let Ok(message) = serde_json::from_str::<Message<ServerTypeResponse>>(&str_data) {
                    // Send to SC
                    if let Some(content) = MessageContent::from_content(message.content.clone()) {
                        self.send_message_received_to_sc(content);
                    }

                    match &message.content {
                        ServerTypeResponse::ServerType(server_type) => {
                            debug!(
                            "Client: {:?} received server type {:?} from {:?}",
                            self.id, server_type, message.source_id
                        );
                            self.server_type_map.insert(message.source_id, Some(server_type.clone()));

                            // remove the session id from the session_ids
                            let remvoed = self.session_ids_for_request_server_type.remove(&message.session_id);
                        }
                    }
                }
                // Try to parse as TextResponse
                else if let Ok(message) = serde_json::from_str::<Message<TextResponse>>(&str_data) {
                    // Send to SC
                    if let Some(content) = MessageContent::from_content(message.content.clone()) {
                        self.send_message_received_to_sc(content);
                    }

                    match message.content {
                        TextResponse::TextList(_file_list) => {
                            debug!("Client: {:?} received TextResponse::TextList from {:?} file list: {:?}", self.id, message.source_id, _file_list);
                        }
                        TextResponse::Text(_file) => {
                            debug!(
                            "Client: {:?} received TextResponse::Text from {:?} file: {:?}",
                            self.id, message.source_id, _file
                        );
                        }
                        TextResponse::NotFound => {
                            debug!(
                            "Client: {:?} received TextResponse::NotFound from {:?}",
                            self.id, message.source_id
                        );
                        }
                    }
                }
                // Try to parse as ChatRequest
                else if let Ok(message) = serde_json::from_str::<Message<ChatResponse>>(&str_data) {
                    match &message.content {
                        ChatResponse::ClientNotRegistered=> {
                            debug!("Client: {:?} received a ClientNotRegistered", self.id);
                            self.send_message_received_to_sc(MessageContent::ChatResponse(ChatResponse::ClientNotRegistered));
                        }
                        ChatResponse::ClientRegistered(node_id) => {
                            debug!("Client: {:?} received a ClientRegistered", self.id);
                            self.send_message_received_to_sc(MessageContent::ChatResponse(ChatResponse::ClientRegistered(*node_id)));
                        }
                        _ => {}
                    }
                }
                // try to parse as media response
                else if let Ok(message) = serde_json::from_str::<Message<MediaResponse>>(&str_data) {
                    // Send to SC
                    if let Some(content) = MessageContent::from_content(message.content.clone()) {
                        self.send_message_received_to_sc(content);
                    }

                    match message.content {
                        MediaResponse::MediaList(_media_list) => {
                            debug!(
                            "Client: {:?} received MediaList from {:?}: {:?}",
                            self.id, message.source_id, _media_list
                        );
                        }
                        MediaResponse::Media(_media_id, _media) => {
                            debug!(
                            "Client: {:?} received full media from media id {:?}: {:?}",
                            self.id, message.source_id, _media_id
                        );
                        }
                        MediaResponse::NotFound => {
                            debug!(
                            "Client: {:?} received NotFound from {:?}",
                            self.id, message.source_id
                        );
                        }
                    }
                } else {
                    debug!(
                    "Client: {:?} received unknown data: {:?}",
                    self.id, str_data
                    );
                }
            }
        }
    }

    fn send_server_type_request(&mut self, server_id: NodeId) {
        // Create a server type request with random session ID
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: ServerTypeRequest::GetServerType,
        };
        self.session_ids_for_request_server_type.insert(session_id);
        debug!(
            "Server: {:?} sending msg to client {:?}, msg: {:?}",
            self.id, server_id, message
        );
        self.send_message_in_fragments(server_id, session_id, message);
    }
    fn send_text_request_text_list(&mut self, server_id: NodeId) {
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: TextRequest::TextList,
        };
        debug!(
            "Server: {:?} sending msg to client {:?}, msg: {:?}",
            self.id, server_id, message
        );
        self.send_message_in_fragments(server_id, session_id, message);
    }
    fn send_text_request_text(&mut self, server_id: NodeId, file_id: u64) {
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: TextRequest::Text(file_id),
        };
        debug!(
            "Server: {:?} sending msg to client {:?}, msg: {:?}",
            self.id, server_id, message
        );
        self.send_message_in_fragments(server_id, session_id, message);
    }

    fn send_registration_request(&mut self, server_id: NodeId) {
        debug!(
            "Client: {:?} requesting registration to server {:?}",
            self.id, server_id
        );

        // Create a registration request with random session ID
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: ChatRequest::Register(self.id),
        };

        self.send_message_in_fragments(server_id, session_id, message);
    }

    fn send_chat_message(&mut self, server_id: NodeId, content: String) {
        debug!(
            "Client: {:?} sending message to server {:?}: {:?}",
            self.id, server_id, content
        );

        // Create a chat message request
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: ChatRequest::SendMessage {
                from: self.id,
                message: content,
            },
        };

        self.send_message_in_fragments(server_id, session_id, message);
    }

    fn send_fragment_to_assembler(&mut self, packet: Packet) -> Result<(), String> {
        // send the packet to the assembler
        match self.assembler_send.send(packet) {
            Ok(_) => {
                debug!("Client: {:?} sent packet to assembler", self.id);
                Ok(())
            }
            Err(e) => {
                debug!(
                    "Client: {:?} failed to send packet to assembler: {}",
                    self.id, e
                );
                Err(format!("Failed to send packet to assembler: {}", e))
            }
        }
    }
}
