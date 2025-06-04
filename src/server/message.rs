
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

