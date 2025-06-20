#[cfg(feature = "debug")]
use crate::debug;

use crate::client_server::network_core::{
    ChatMessage, ClientServerCommand, NetworkNode, ServerEvent, ServerType,
};
use crate::message::message::*;
use crossbeam_channel::{select_biased, Receiver, Sender};
use rand::random;
use std::collections::{HashMap, HashSet, VecDeque};
use wg_2024::controller::DroneCommand;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, NodeType, Packet, PacketType};

pub struct CommunicationServer {
    id: NodeId,
    topology_map: HashSet<(NodeId, Vec<NodeId>)>,
    connected_drone_ids: Vec<NodeId>,
    controller_send: Sender<ServerEvent>,
    controller_recv: Receiver<ClientServerCommand>,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    packet_recv: Receiver<Packet>,
    assembler_send: Sender<Packet>,
    // assembler_recv: Receiver<Packet>,
    assembler_res_recv: Receiver<Vec<u8>>,
    // assembler_res_send: Sender<Vec<u8>>,
    registered_clients: HashSet<NodeId>,
    messages_stored: Vec<ChatMessage>,
}

impl NetworkNode for CommunicationServer {
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
        debug!(
            "Communication Server: {:?} started and waiting for packets",
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

impl CommunicationServer {
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
        registered_clients: HashSet<NodeId>,
        messages_stored: Vec<ChatMessage>,
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
            registered_clients,
            messages_stored,
        }
    }

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
            }
            ClientServerCommand::SendChatMessage(_target_id, _msg) => {
                debug!(
                    "Server: {:?} sending chat message to {:?}: {:?}",
                    self.id, _target_id, _msg
                );
            }
            ClientServerCommand::StartFloodRequest => {
                debug!("Server: {:?} received StartFloodRequest command", self.id);

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
                    \nChat Server: {:?}\
                    \ntopology_map: {:?}\
                    \nregistered_clients: {:?}\
                    \nmessage_store: {:?}\
                    \n",
                    self.id, self.topology_map, self.registered_clients, self.messages_stored
                );
            }
        }
    }
    fn handle_packet(&mut self, packet: Packet) {
        match &packet.pack_type {
            PacketType::Nack(_nack) => {
                debug!("Server: {:?} received a Ack {:?}", self.id, _nack);
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
                        debug!("Com Server: {:?} sent fragment to assembler", self.id);

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
                self.update_topology_with_flood_response(_flood_response);
            }
        }
    }
    fn handle_assembler_data(&mut self, data: Vec<u8>) {
        if let Ok(str_data) = String::from_utf8(data.clone()) {
            debug!(
                "Server {:?} received assembled message: {:?}",
                self.id, str_data
            );

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
                        self.send_text_response(message.source_id, message.session_id);
                    }
                    TextRequest::Text(_file_id) => {
                        debug!(
                            "Server: {:?} received TextRequest::Text from {:?} file id: {:?}",
                            self.id, message.source_id, _file_id
                        );
                        self.send_text_response(message.source_id, message.session_id);
                    }
                }
            }
            // Then try to parse as ChatRequest
            else if let Ok(message) = serde_json::from_str::<Message<ChatRequest>>(&str_data) {
                // Send to SC
                if let Some(content) = MessageContent::from_content(message.content.clone()) {
                    self.send_message_received_to_sc(content);
                }

                match message.content {
                    ChatRequest::Register(client_id) => {
                        debug!("Server: {:?} received registration request from client {:?}", self.id, client_id);

                        self.registered_clients.insert(client_id);  // Insert client in registered_clients.

                        let chat_message = ChatMessage {
                            sender_id: client_id,
                            content: String::from(format!("Client {} has entered the chatroom", client_id)),
                        };

                        self.messages_stored.push(chat_message);

                        // Respond to client with ClientRegistered
                        let session_id = random::<u64>();
                        let message = Message {
                            source_id: self.id,
                            session_id,
                            content: ChatResponse::ClientRegistered(self.id),
                        };

                        self.send_message_in_fragments(client_id, session_id, message);

                        debug!("Server: {:?} now has registered client: {:?}", self.id, client_id);
                    }

                    ChatRequest::ClientList => {
                        debug!("Server: {:?} received ClientList request from {:?}", self.id, message.source_id);

                        self.send_server_client_list(message.source_id);
                    }

                    ChatRequest::SendMessage { from, message } => {
                        debug!("Server: {:?} received SendMessage request from {:?}", self.id, from);

                        self.handle_incoming_message(from, message);
                    }
                }
            }
        }
    }

    fn send_server_type_response(&mut self, client_id: NodeId, _session_id: u64) {
        // Create response message with Communication server type
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: ServerTypeResponse::ServerType(ServerType::CommunicationServer),
        };

        debug!("Server: {:?} sending msg to client {:?}, msg: {:?}", self.id, client_id, message);
        self.send_message_in_fragments(client_id, session_id, message);
    }
    
    fn send_text_response(&mut self, client_id: NodeId, _session_id: u64) {
        debug!("Server: {:?} is a chat server!", self.id);

        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: TextResponse::NotFound,
        };

        debug!("Server: {:?} sending msg to client {:?}, msg: {:?}", self.id, client_id, message);
        self.send_message_in_fragments(client_id, session_id, message);
    }

    fn send_server_client_list(&mut self, client_id: NodeId) {

        // Create response message with the client list
        let session_id = random::<u64>();
        let message = Message {
            source_id: self.id,
            session_id,
            content: ChatResponse::ClientList(self.registered_clients.clone()),
        };

        debug!("Server: {:?} sending client list to {:?}", self.id, client_id);
        self.send_message_in_fragments(client_id, session_id, message);
    }

    fn handle_incoming_message(&mut self, client_id: NodeId, content: String) {
        // Check if the sender is registered
        if !self.registered_clients.contains(&client_id) {
            debug!("Server: {:?} received message from unregistered client {:?}", self.id, client_id);

            //If not registered send message with ClientNotRegistered
            let session_id = random::<u64>();
            let message = Message {
                source_id: self.id,
                session_id,
                content: ChatResponse::ClientNotRegistered,
            };

            debug!("Server: {:?} sending ClientNotRegistered to client {:?}, msg: {:?}", self.id, client_id, message);
            self.send_message_in_fragments(client_id, session_id, message);
            return;
        }

        // If client is registered, store the message.
        let chat_message = ChatMessage {
            sender_id: client_id,
            content,
        };

        debug!("Server: {:?} storing message from {:?}", self.id, client_id);

        self.messages_stored.push(chat_message);

        // Sends to simulation controller the whole chatroom.
        self.send_message_received_to_sc(MessageContent::WholeChatVecResponse(Chatroom{
            server_id: self.id,
            chatroom_messages: self.messages_stored.clone(),
        }));
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
