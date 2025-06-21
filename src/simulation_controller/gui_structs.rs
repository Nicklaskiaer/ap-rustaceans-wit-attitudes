use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneEvent};
use wg_2024::network::NodeId;
use crate::client_server::network_core::{ClientEvent, ServerEvent};

pub struct ClientsDownloadedData {
    data: HashMap<(NodeId, NodeId), HashSet<u64>>, // (client, server) -> list of index
    known_data: HashMap<u64, bool>, // index -> has been downloaded?
}

impl ClientsDownloadedData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            known_data: HashMap::new(),
        }
    }

    pub fn add_list(&mut self, client_id: NodeId, server_id: NodeId, index_list: HashSet<u64>) {
        // Get or create the HashSet for this client-server pair
        let client_server_data = self.data.entry((client_id, server_id)).or_insert_with(HashSet::new);

        // Add all indices from the new list
        for index in &index_list {
            client_server_data.insert(*index);
            // Add to known_data with false if not already present
            self.known_data.entry(*index).or_insert(false);
        }
    }
    pub fn add_data(&mut self, client_id: NodeId, server_id: NodeId, data_index: Vec<u64>) {
        // Get or create the HashSet for this client-server pair
        let client_server_data = self.data.entry((client_id, server_id)).or_insert_with(HashSet::new);

        // Add all indices from the data_index vector
        for index in &data_index {
            client_server_data.insert(*index);
            // Mark as downloaded (true)
            self.known_data.insert(*index, true);
        }
    }

    pub fn get_know_list(&self, client_id: NodeId, server_id: NodeId) -> Option<Vec<u64>> {
        // Get the HashSet for this client-server pair, regardless of download status
        self.data.get(&(client_id, server_id))
            .map(|hashset| hashset.iter().copied().collect())
    }
    pub fn get_know_data(&self, client_id: NodeId, server_id: NodeId) -> Option<Vec<u64>> {
        // Get the HashSet for this client-server pair, but only return downloaded data (true)
        self.data.get(&(client_id, server_id))
            .map(|hashset| {
                hashset.iter()
                    .filter(|&index| self.known_data.get(index).copied().unwrap_or(false))
                    .copied()
                    .collect()
            })
    }
    pub fn get_all_know_data(&self, client_id: NodeId) -> Option<Vec<u64>> {
        // Find all servers this client has data from
        let client_data: HashSet<u64> = self.data.iter()
            .filter_map(|((c_id, _), indices)| {
                if *c_id == client_id {
                    Some(indices)
                } else {
                    None
                }
            })
            .flat_map(|indices| indices.iter())
            .copied()
            .collect();

        if client_data.is_empty() {
            None
        } else {
            // Filter to only include downloaded data (true in known_data)
            let downloaded_data: Vec<u64> = client_data.iter()
                .filter(|&index| self.known_data.get(index).copied().unwrap_or(false))
                .copied()
                .collect();

            if downloaded_data.is_empty() {
                None
            } else {
                Some(downloaded_data)
            }
        }
    }
}

#[derive(PartialEq)]
pub enum Screen {
    NetworkScreen,
    LogsScreen,
}

#[derive(PartialEq)]
pub enum ClientPopupScreen {
    Chatroom,
    Content,
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