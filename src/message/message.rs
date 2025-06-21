use crate::client_server::network_core::{ChatMessage, ServerType};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use wg_2024::network::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "M: DeserializeOwned"))]
pub struct Message<M: DroneSend> {
    pub source_id: NodeId,
    pub session_id: u64,
    pub content: M,
}

// Used in handle_assembler_data for both client and server
pub trait DroneSend: Serialize + DeserializeOwned + std::fmt::Debug {
    fn stringify(&self) -> String;
    fn from_string(raw: String) -> Result<Self, String>;
}

pub trait Request: DroneSend {
    fn request_type(&self) -> String;
}
pub trait Response: DroneSend {
    fn response_type(&self) -> String;
}

// Wrapper for the ClientEvent and ServerEvent
#[derive(Debug, Clone)]
pub enum MessageContent {
    ServerTypeRequest(ServerTypeRequest),
    ServerTypeResponse(ServerTypeResponse),
    TextRequest(TextRequest),
    TextResponse(TextResponse),
    WholeChatVecResponse(Chatroom),
    ChatRequest(ChatRequest),
    ChatResponse(ChatResponse),
    MediaRequest(MediaRequest),
    MediaResponse(MediaResponseForMessageContent),
    MediaListWithServer(NodeId, Vec<u64>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaResponseForMessageContent {
    MediaList(Vec<u64>),
    Media(u64),
    NotFound,
}

impl MediaResponseForMessageContent {
    pub fn new(media_response: MediaResponse) -> Self {
        match &media_response {
            MediaResponse::MediaList(_m) => Self::MediaList(_m.clone()),
            MediaResponse::Media(_m, _) => Self::Media(_m.clone()),
            MediaResponse::NotFound => Self::NotFound,
        }
    }
}

impl MessageContent {
    // Converts a message content into a MessageContent enum variant
    pub fn from_content<T: DroneSend>(content: T) -> Option<Self> {
        // Try type conversions for each supported message type
        if let Ok(content) = serde_json::to_value(&content)
            .and_then(|v| serde_json::from_value::<ServerTypeRequest>(v))
        {
            Some(MessageContent::ServerTypeRequest(content))
        } else if let Ok(content) = serde_json::to_value(&content)
            .and_then(|v| serde_json::from_value::<ServerTypeResponse>(v))
        {
            Some(MessageContent::ServerTypeResponse(content))
        } else if let Ok(content) =
            serde_json::to_value(&content).and_then(|v| serde_json::from_value::<TextRequest>(v))
        {
            Some(MessageContent::TextRequest(content))
        } else if let Ok(content) =
            serde_json::to_value(&content).and_then(|v| serde_json::from_value::<TextResponse>(v))
        {
            Some(MessageContent::TextResponse(content))
        } else if let Ok(content) =
            serde_json::to_value(&content).and_then(|v| serde_json::from_value::<MediaRequest>(v))
        {
            Some(MessageContent::MediaRequest(content))
        } else if let Ok(content) = serde_json::to_value(&content)
            .and_then(|v| serde_json::from_value::<MediaResponseForMessageContent>(v))
        {
            Some(MessageContent::MediaResponse(content))
        } else {
            None
        }
    }
}

// ReqServerType,
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextRequest {
    TextList,
    Text(u64),
}

impl DroneSend for TextRequest {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

impl Request for TextRequest {
    fn request_type(&self) -> String {
        match self {
            TextRequest::TextList => "TextList".to_string(),
            TextRequest::Text(_) => "Text".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaRequest {
    MediaList,
    Media(u64),
}

impl DroneSend for MediaRequest {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

impl Request for MediaRequest {
    fn request_type(&self) -> String {
        match self {
            MediaRequest::MediaList => "MediaList".to_string(),
            MediaRequest::Media(_) => "Media".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRequest {
    ClientList,
    Register(NodeId),
    SendMessage { from: NodeId, message: String },
}

impl DroneSend for ChatRequest {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}
impl Request for ChatRequest {
    fn request_type(&self) -> String {
        match self {
            ChatRequest::ClientList => "ClientList".to_string(),
            ChatRequest::Register(_) => "Register".to_string(),
            ChatRequest::SendMessage { .. } => "SendMessage".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextResponse {
    TextList(Vec<u64>),
    Text(String),
    NotFound,
}

impl DroneSend for TextResponse {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

impl Response for TextResponse {
    fn response_type(&self) -> String {
        match self {
            TextResponse::TextList(_) => "TextList".to_string(),
            TextResponse::Text(_) => "Text".to_string(),
            TextResponse::NotFound => "NotFound".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaResponse {
    MediaList(Vec<u64>),
    Media(u64, Vec<u8>),
    NotFound,
}

impl DroneSend for MediaResponse {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}
impl Response for MediaResponse {
    fn response_type(&self) -> String {
        match self {
            MediaResponse::MediaList(_) => "MediaList".to_string(),
            MediaResponse::Media(_, _) => "Media".to_string(),
            MediaResponse::NotFound => "NotFound".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatResponse {
    ClientList(HashSet<NodeId>),
    MessageFrom { from: NodeId, message: Vec<u8> },
    MessageSent,
    ClientNotRegistered,
    ClientRegistered(NodeId),
}

#[derive(Clone, Debug)]
pub struct Chatroom {
    pub server_id: NodeId,
    pub chatroom_messages: Vec<ChatMessage>,
}

impl DroneSend for ChatResponse {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

impl Response for ChatResponse {
    fn response_type(&self) -> String {
        match self {
            ChatResponse::ClientList(_) => "ClientList".to_string(),
            ChatResponse::MessageFrom { .. } => "MessageFrom".to_string(),
            ChatResponse::MessageSent => "MessageSent".to_string(),
            ChatResponse::ClientNotRegistered => "ClientNotRegistered".to_string(),
            ChatResponse::ClientRegistered(_) => "ClientRegistered".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerTypeRequest {
    GetServerType,
}

impl DroneSend for ServerTypeRequest {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

impl Request for ServerTypeRequest {
    fn request_type(&self) -> String {
        "GetServerType".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerTypeResponse {
    ServerType(ServerType),
}

impl DroneSend for ServerTypeResponse {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

impl Response for ServerTypeResponse {
    fn response_type(&self) -> String {
        "GetServerType".to_string()
    }
}
