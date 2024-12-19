use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use wg_2024::network::NodeId;

#[derive(Debug, Clone)]
pub struct Message<M: DroneSend> {
    pub source_id: NodeId,
    pub session_id: u64,
    pub content: M,
}

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
    SendMessage {
        from: NodeId,
        to: NodeId,
        message: String,
    },
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
    Media(Vec<u8>), // should we use some other type?
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
            MediaResponse::Media(_) => "Media".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatResponse {
    ClientList(Vec<NodeId>),
    MessageFrom { from: NodeId, message: Vec<u8> },
    MessageSent,
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
        }
    }
}
