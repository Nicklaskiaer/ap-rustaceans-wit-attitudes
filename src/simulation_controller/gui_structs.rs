use wg_2024::controller::{DroneEvent};
use crate::client::client::ClientEvent;
use crate::server::server::ServerEvent;

#[derive(PartialEq)]
pub enum Screen {
    NetworkScreen,
    LogsScreen,
}

#[derive(PartialEq)]
pub enum ClientPopupScreen {
    Chatroom,
    Other,
}

pub struct LogEntry {
    pub(crate) timestamp: String,
    pub(crate) message: String,
}

pub enum Event{
    Drone(DroneEvent),
    Client(ClientEvent),
    Server(ServerEvent),
}

pub struct Node {
    pub(crate) id: String,
    pub(crate) position: (f32, f32),
    pub(crate) is_client: bool,
    pub(crate) is_server: bool,
}