pub enum ClientToServer {
    ServerType,
    FilesList,
    File { file_id: String },
    Media { media_id: String },
}

pub enum ServerToClient {
    ServerType { server_type: String },
    FilesList { list_of_file_ids: Vec<String> },
    File { file_size: usize, file: Vec<u8> },
    Media { media: Vec<u8> },
    ErrorRequestedNotFound,
    ErrorUnsupportedRequest,
}

pub enum ChatClientToServer {
    RegistrationToChat,
    ClientList,
    MessageFor { client_id: String, message: String },
}

pub enum ChatServerToClient {
    ClientList { list_of_client_ids: Vec<String> },
    MessageFrom { client_id: String, message: String },
    ErrorWrongClientId,
}

impl ClientToServer {
    pub fn handle_request(&self) -> ServerToClient {
        match self {
            ClientToServer::ServerType => ServerToClient::ServerType {
                server_type: "ExampleServer".to_string(),
            },
            ClientToServer::FilesList => ServerToClient::FilesList {
                list_of_file_ids: vec!["file1".to_string(), "file2".to_string()],
            },
            ClientToServer::File { file_id } => ServerToClient::File {
                file_size: 1024,
                file: vec![0; 1024],
            },
            ClientToServer::Media { media_id } => ServerToClient::Media {
                media: vec![1, 2, 3, 4],
            },
        }
    }
}

impl ChatClientToServer {
    pub fn handle_request(&self) -> ChatServerToClient {
        match self {
            ChatClientToServer::RegistrationToChat => ChatServerToClient::ClientList {
                list_of_client_ids: vec!["client1".to_string(), "client2".to_string()],
            },
            ChatClientToServer::ClientList => ChatServerToClient::ClientList {
                list_of_client_ids: vec!["client1".to_string(), "client2".to_string()],
            },
            ChatClientToServer::MessageFor { client_id, message } => {
                ChatServerToClient::MessageFrom {
                    client_id: client_id.clone(),
                    message: message.clone(),
                }
            }
        }
    }
}

impl ServerToClient {
    pub fn handle_response(&self) {
        match self {
            ServerToClient::ServerType { server_type } => println!("Server type: {}", server_type),
            ServerToClient::FilesList { list_of_file_ids } => {
                println!("Files list: {:?}", list_of_file_ids)
            }
            ServerToClient::File { file_size, file } => {
                println!("File size: {}, File content: {:?}", file_size, file)
            }
            ServerToClient::Media { media } => println!("Media content: {:?}", media),
            ServerToClient::ErrorRequestedNotFound => println!("Error: Requested item not found"),
            ServerToClient::ErrorUnsupportedRequest => println!("Error: Unsupported request"),
        }
    }
}

impl ChatServerToClient {
    pub fn handle_response(&self) {
        match self {
            ChatServerToClient::ClientList { list_of_client_ids } => {
                println!("Client list: {:?}", list_of_client_ids)
            }
            ChatServerToClient::MessageFrom { client_id, message } => {
                println!("Message from {}: {}", client_id, message)
            }
            ChatServerToClient::ErrorWrongClientId => println!("Error: Wrong client ID"),
        }
    }
}
