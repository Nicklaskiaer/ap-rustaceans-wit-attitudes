use wg_2024::controller::DroneCommand;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use crossbeam_channel::Sender;

pub enum ClientServerCommand {
    StartFloodRequest,
    RegistrationRequest(NodeId),
    RequestServerList(NodeId), //list of all commenced clients
    RequestFileList(NodeId),
    SendChatMessage(NodeId, usize, String),

    // Drone commands
    DroneCmd(DroneCommand),
}

impl From<DroneCommand> for ClientServerCommand {
    fn from(cmd: DroneCommand) -> Self {
        ClientServerCommand::DroneCmd(cmd)
    }
}