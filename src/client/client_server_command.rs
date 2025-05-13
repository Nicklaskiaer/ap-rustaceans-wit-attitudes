use wg_2024::controller::DroneCommand;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use crossbeam_channel::Sender;

pub enum ClientServerCommand {
    StartFloodRequest,
    // RegistrationRequest(NodeId),
    // RequestServerType(NodeId), // client request the server type
    // ResponseServerType(NodeId), // server send its server type
    // RequestServerList(NodeId), // client request the server a list of all connected clients
    // RequestFileList(NodeId),
    SendChatMessage(NodeId, usize, String),

    // Drone commands
    DroneCmd(DroneCommand),
}

impl From<DroneCommand> for ClientServerCommand {
    fn from(cmd: DroneCommand) -> Self {
        ClientServerCommand::DroneCmd(cmd)
    }
}