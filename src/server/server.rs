use crate::server::message::*;
use wg_2024::network::*;

pub struct ContentServer;
pub struct CommunicationServer;

#[derive(Debug, Clone)]
pub enum ServerType {
    Content,
    CommunicationServer,
}

pub trait Server {
    type RequestType: Request;
    type ResponseType: Response;

    fn get_server_type(&self) -> ServerType;
    fn handle_request(&self, message: Message<TextRequest>) -> Message<TextRequest>;
    fn send_response(&self, message: Message<impl DroneSend>);
    // fn compute_route(&self, hops: Vec<i32>, hop_index: usize) -> Vec<i32>;
    fn create_source_routing_header(
        &self,
        hops: Vec<NodeId>,
        hop_index: usize,
    ) -> SourceRoutingHeader;
    fn compose_message(
        source_id: NodeId,
        session_id: u64,
        raw_content: String,
    ) -> Result<Message<Self::RequestType>, String> {
        let content = Self::RequestType::from_string(raw_content)?;
        Ok(Message {
            session_id,
            source_id,
            content,
        })
    }
}

impl Server for ContentServer {
    type RequestType = TextRequest;
    type ResponseType = TextResponse;

    fn get_server_type(&self) -> ServerType {
        ServerType::Content
    }

    fn handle_request(&self, message: Message<TextRequest>) -> Message<TextRequest> {
        match message.content {
            TextRequest::TextList => {
                let response = TextRequest::Text(1);
                Message {
                    source_id: message.source_id,
                    session_id: message.session_id,
                    content: response,
                }
            }
            TextRequest::Text(_) => {
                let response = TextRequest::Text(1);
                Message {
                    source_id: message.source_id,
                    session_id: message.session_id,
                    content: response,
                }
            }
        }
    }

    fn send_response(&self, message: Message<impl DroneSend>) {
        println!("Sending response: {:?}", message);
    }

    fn create_source_routing_header(
        &self,
        hops: Vec<NodeId>,
        hop_index: usize,
    ) -> SourceRoutingHeader {
        SourceRoutingHeader::new(hops, hop_index)
    }
}

impl Server for CommunicationServer {
    type RequestType = TextRequest;
    type ResponseType = TextResponse;

    fn get_server_type(&self) -> ServerType {
        ServerType::CommunicationServer
    }

    fn handle_request(&self, message: Message<TextRequest>) -> Message<TextRequest> {
        match message.content {
            TextRequest::TextList => {
                let response = TextRequest::Text(1);
                Message {
                    source_id: message.source_id,
                    session_id: message.session_id,
                    content: response,
                }
            }
            TextRequest::Text(_) => {
                let response = TextRequest::Text(1);
                Message {
                    source_id: message.source_id,
                    session_id: message.session_id,
                    content: response,
                }
            }
        }
    }

    fn send_response(&self, message: Message<impl DroneSend>) {
        println!("Sending response: {:?}", message);
    }

    fn create_source_routing_header(
        &self,
        hops: Vec<NodeId>,
        hop_index: usize,
    ) -> SourceRoutingHeader {
        SourceRoutingHeader::new(hops, hop_index)
    }
}
