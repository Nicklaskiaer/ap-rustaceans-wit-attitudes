use wg_2024::controller::{DroneEvent};
use crate::client_server::network_core::{ClientEvent, ServerEvent};

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

pub struct LogFilters {
    pub show_events: bool,
    pub show_commands: bool,
    pub show_drones: bool,
    pub show_clients: bool,
    pub show_servers: bool,
    pub search_text: String,
}

impl Default for LogFilters {
    fn default() -> Self {
        Self{
            show_events: true,
            show_commands: true,
            show_drones: true,
            show_clients: true,
            show_servers: true,
            search_text: String::new(),
        }
    }
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