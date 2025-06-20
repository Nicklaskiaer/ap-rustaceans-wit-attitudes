#[cfg(feature = "debug")]
use crate::debug;

use crate::assembler::assembler::*;
use crate::client_server::network_core::{
    ClientServerCommand, ContentType, NetworkNode, ServerEvent, ServerType,
};
use crate::message::message::*;
use crossbeam_channel::{select_biased, Receiver, SendError, Sender};
use rand::random;
use std::collections::{HashMap, HashSet};
use std::thread;
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, NodeType, Packet, PacketType};

pub struct ContentServer {
    id: NodeId,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ServerEvent>,
    controller_recv: Receiver<ClientServerCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    assembler_send: Sender<Packet>,
    assembler_res_recv: Receiver<Vec<u8>>,
    content_type: ContentType,
    files: Vec<u64>,
}

impl NetworkNode for ContentServer {
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

    fn run(&mut self) {
        debug!(
            "Content Server: {:?} started and waiting for packets",
            self.id
        );
        loop {
            select_biased! {
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        self.handle_command(command);
                    }
                },
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        self.send_packet_received_to_sc(packet.clone());
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
            .send(ServerEvent::PacketSent(packet))
            .expect("this is fine 🔥☕");
    }
    fn send_packet_received_to_sc(&mut self, packet: Packet) {
        self.controller_send
            .send(ServerEvent::PacketReceived(packet))
            .expect("this is fine 🔥☕");
    }
    fn send_message_sent_to_sc(&mut self, message: MessageContent, target: NodeId) {
        self.controller_send
            .send(ServerEvent::MessageSent {
                from: self.id,
                to: target,
                content: message,
            })
            .expect("this is fine 🔥☕");
    }
    fn send_message_received_to_sc(&mut self, message: MessageContent) {
        self.controller_send
            .send(ServerEvent::MessageReceived {
                receiver: self.id,
                content: message })
            .expect("this is fine 🔥☕");
    }
}

impl ContentServer {
    pub fn new(
        id: NodeId,
        connected_drone_ids: Vec<NodeId>,
        controller_send: Sender<ServerEvent>,
        controller_recv: Receiver<ClientServerCommand>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        topology_map: HashSet<(NodeId, Vec<NodeId>)>,
        assembler_send: Sender<Packet>,
        // assembler_recv: Receiver<Packet>,
        // assembler_res_send: Sender<Vec<u8>>,
        assembler_res_recv: Receiver<Vec<u8>>,
        content_type: ContentType,
        files: Vec<u64>,
    ) -> Self {
        Self {
            id,
            connected_drone_ids,
            controller_send,
            controller_recv,
            packet_recv,
            packet_send,
            topology_map,
            assembler_send,
            // assembler_recv,
            // assembler_res_send,
            assembler_res_recv,
            content_type,
            files,
        }
    }

    fn handle_command(&mut self, command: ClientServerCommand) {
        match command {
            ClientServerCommand::SendChatMessage(_node_id, _msg) => {
                debug!(
                    "Server: {:?} received SendChatMessage command for node {:?}: {:?}",
                    self.id, _node_id, _msg
                );
            }
            ClientServerCommand::StartFloodRequest => {
                debug!("Server: {:?} received StartFloodRequest command", self.id);

                // clear the hashmap
                self.topology_map.clear();

                // Generate a unique flood ID using current time
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let flood_id = timestamp ^ random::<u64>();

                // Create path trace with just this server
                let path_trace = vec![(self.id, NodeType::Server)];

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
            }
            ClientServerCommand::RequestServerType => { /* servers do not need to use it */ }
            ClientServerCommand::RequestFileList(_) => { /* servers do not need to use it */ }
            ClientServerCommand::RequestFile(_, _) => { /* servers do not need to use it */ }
            ClientServerCommand::RegistrationRequest(_) => { /* this server do not need to use it */
            }
            ClientServerCommand::TestCommand => {
                debug!(
                    "\n\
                    \nContent Server: {:?}\
                    \ntopology_map: {:?}\
                    \ncontent_type: {:?}\
                    \nfiles: {:?}\
                    \n",
                    self.id, self.topology_map, self.content_type, self.files
                );
            }
            ClientServerCommand::ClientListRequest(_) => {/* servers do not need to use it */ },
            ClientServerCommand::RemoveDrone(drone_id) => {
                self.connected_drone_ids.retain(|&id| id != drone_id);
            }
        }
    }
    fn handle_packet(&mut self, packet: Packet) {
        match &packet.pack_type {
            PacketType::Nack(_nack) => {
                debug!("Server: {:?} received a FloodResponse {:?}", self.id, _nack);
            }
            PacketType::Ack(_ack) => {
                debug!("Server: {:?} received a Ack {:?}", self.id, _ack);
            }
            PacketType::MsgFragment(_fragment) => {
                debug!(
                    "Server: {:?} received a MsgFragment {:?}",
                    self.id, _fragment
                );

                // Send fragment to assembler to be reassembled
                match self.send_fragment_to_assembler(packet.clone()) {
                    Ok(_) => {
                        debug!("Content Server: {:?} sent fragment to assembler", self.id);

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
            }
            PacketType::FloodRequest(_flood_request) => {
                debug!(
                    "Server: {:?} received a FloodRequest {:?}",
                    self.id, _flood_request
                );

                // send a flood response
                // add node to the path trace
                let mut flood_request = _flood_request.clone();
                flood_request.increment(self.id, NodeType::Server);
                // generate a flood response
                let mut flood_response_packet = flood_request.generate_response(packet.session_id);
                debug!(
                    "Server: {:?} is generating a FloodResponse: {:?}",
                    self.id, flood_response_packet
                );
                flood_response_packet.routing_header.increase_hop_index();

                // Try to send packet
                self.try_send_packet(&flood_response_packet);
            }
            PacketType::FloodResponse(_flood_response) => {
                debug!(
                    "Server: {:?} received a FloodResponse {:?}",
                    self.id, _flood_response
                );
                self.update_topology_with_flood_response(_flood_response, false);
            }
        }
    }
    fn handle_assembler_data(&mut self, data: Vec<u8>) {
        if let Ok(str_data_raw) = String::from_utf8(data.clone()) {
            debug!(
                "Server {:?} received assembled message: {:?}",
                self.id, str_data_raw
            );

            if let Some(str_data) = str_data_raw.split("\0").nth(0) {
                // Try to parse as ServerTypeRequest
                if let Ok(message) = serde_json::from_str::<Message<ServerTypeRequest>>(&str_data) {
                    // Send to SC
                    if let Some(content) = MessageContent::from_content(message.content.clone()) {
                        self.send_message_received_to_sc(content);
                    }

                    match message.content {
                        ServerTypeRequest::GetServerType => {
                            debug!(
                            "Server: {:?} received ServerTypeRequest from {:?}",
                            self.id, message.source_id
                        );
                            self.send_server_type_response(message.source_id, message.session_id);
                        }
                    }
                }
                // Try to parse as TextRequest
                else if let Ok(message) = serde_json::from_str::<Message<TextRequest>>(&str_data) {
                    // Send to SC
                    if let Some(content) = MessageContent::from_content(message.content.clone()) {
                        self.send_message_received_to_sc(content);
                    }

                    match message.content {
                        TextRequest::TextList => {
                            debug!(
                            "Server: {:?} received TextRequest::TextList from {:?}",
                            self.id, message.source_id
                        );
                            self.send_text_response_text_list(message.source_id);
                        }
                        TextRequest::Text(file_id) => {
                            debug!(
                            "Server: {:?} received TextRequest::Text from {:?} file id: {:?}",
                            self.id, message.source_id, file_id
                        );
                            self.send_text_response_text(message.source_id, file_id);
                        }
                    }
                } else if let Ok(message) = serde_json::from_str::<Message<MediaRequest>>(&str_data) {
                    // Send to SC
                    if let Some(content) = MessageContent::from_content(message.content.clone()) {
                        self.send_message_received_to_sc(content);
                    }

                    match message.content {
                        MediaRequest::MediaList => {
                            self.handle_media_list_request(message.source_id);
                        }
                        MediaRequest::Media(file_nr) => {
                            self.handle_media_request(message.source_id, file_nr);
                        }
                    }
                } else {
                    debug!(
                    "Server: {:?} received unknown message format: {:?}",
                    self.id, str_data
                    );
                }
            }
        }
    }

    fn send_server_type_response(&mut self, client_id: NodeId, session_id: u64) {
        // Create response message with Communication server type
        let message = Message {
            source_id: self.id,
            session_id,
            content: ServerTypeResponse::ServerType(ServerType::ContentServer(
                self.content_type.clone(),
            )),
        };
        debug!(
            "Server: {:?} sending msg to client {:?}, msg: {:?}",
            self.id, client_id, message
        );
        self.send_message_in_fragments(client_id, session_id, message);
    }
    fn send_text_response_text_list(&mut self, client_id: NodeId) {
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: TextResponse::TextList(self.files.clone()),
        };
        debug!(
            "Server: {:?} sending msg to client {:?}, msg: {:?}",
            self.id, client_id, message
        );
        self.send_message_in_fragments(client_id, session_id, message);
    }
    fn send_text_response_text(&mut self, client_id: NodeId, file_id: u64) {
        if self.files.contains(&file_id) {
            // Try to read file content
            let file_path = format!("server_content/text_files/{}", file_id);
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    let session_id = random::<u64>();
                    let message = Message {
                        source_id: self.id,
                        session_id,
                        content: TextResponse::Text(content),
                    };
                    debug!(
                        "Server: {:?} sending msg to client {:?}, msg: {:?}",
                        self.id, client_id, message
                    );
                    self.send_message_in_fragments(client_id, session_id, message);
                }
                Err(_e) => {
                    debug!(
                        "Server: {:?} failed to read file {:?}: {}",
                        self.id, file_id, _e
                    );
                    let session_id = random::<u64>();
                    let message = Message {
                        source_id: self.id,
                        session_id,
                        content: TextResponse::NotFound,
                    };
                    debug!(
                        "Server: {:?} sending msg to client {:?}, msg: {:?}",
                        self.id, client_id, message
                    );
                    self.send_message_in_fragments(client_id, session_id, message);
                }
            }
        } else {
            debug!("Server: {:?} doesn't have {:?}", self.id, file_id);
            let session_id = random::<u64>();
            let message = Message {
                source_id: self.id,
                session_id,
                content: TextResponse::NotFound,
            };
            debug!(
                "Server: {:?} sending msg to client {:?}, msg: {:?}",
                self.id, client_id, message
            );
            self.send_message_in_fragments(client_id, session_id, message);
        }
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

    fn handle_media_list_request(&mut self, message_id: NodeId) {
        debug!(
            "Server: {:?} received MediaRequest::MediaList from {:?}",
            self.id, message_id
        );
        // Handle MediaList request
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: MediaResponse::MediaList(self.files.clone()),
        };
        debug!(
            "Server: {:?} sending MediaList response to client {:?}, msg: {:?}",
            self.id, message_id, message
        );
        self.send_message_in_fragments(message_id, session_id, message);
    }

    fn handle_media_request(&mut self, message_id: NodeId, file_nr: u64) {
        debug!(
            "Server: {:?} received MediaRequest::Media from {:?} file id: {:?}",
            self.id, message_id, file_nr
        );
        // Handle Media request
        if self.files.contains(&file_nr) {
            let session_id = random::<u64>();
            let message = Message {
                source_id: self.id,
                session_id,
                content: MediaResponse::Media(
                    file_nr,
                    std::fs::read(format!("server_content/media_files/{}.jpg", file_nr))
                        .unwrap_or_else(|_e| {
                            debug!(
                                "Server: {:?} failed to read media file {:?}: {}",
                                self.id, file_nr, _e
                            );
                            vec![]
                        }),
                ),
            };
            debug!(
                "Server: {:?} sending Media response to client {:?}, msg: {:?}",
                self.id, message_id, message
            );
            self.send_message_in_fragments(message_id, session_id, message);
        } else {
            debug!(
                "Server: {:?} does not have media file {:?}",
                self.id, file_nr
            );
            let session_id = random::<u64>();
            let message = Message {
                source_id: self.id,
                session_id,
                content: MediaResponse::NotFound,
            };
            debug!(
                "Server: {:?} sending NotFound response to client {:?}, msg: {:?}",
                self.id, message_id, message
            );
            self.send_message_in_fragments(message_id, session_id, message);
        }
    }
}
