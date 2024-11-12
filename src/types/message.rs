use crate::types::NodeId;
pub struct Message {
    message_data: MessageData,
    routing_header: SourceRoutingHeader,
}

pub struct MessageData {
    source_id: NodeId,
    session_id: u64,
    content: MessageContent,
}

pub enum ServerType {
    Chat,
    Text,
    Media
}

pub enum MessageContent {
    // Client -> Server
    ReqServerType,
    ReqFilesList,
    ReqFile(u64),
    ReqMedia(u64),

    ReqClientList,
    ReqMessageSend { to: NodeId, message: Vec<u8> },

    // Server -> Client
    RespServerType(ServerType),
    RespFilesList(Vec<u64>),
    RespFile(Vec<u8>),
    RespMedia(Vec<u8>),
    ErrUnsupportedRequestType,
    ErrRequestedNotFound,

    RespClientList(Vec<NodeId>),
    RespMessageFrom { from: NodeId, message: Vec<u8> },
    ErrWrongClientId,
}

