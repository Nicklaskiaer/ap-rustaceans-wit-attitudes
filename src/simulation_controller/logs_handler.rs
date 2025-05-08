use wg_2024::controller::{DroneCommand, DroneEvent};

use crate::simulation_controller::gui_structs::*;
use crate::client::client::ClientEvent;
use crate::server::server::ServerEvent;

use chrono::{DateTime, Local, Utc};
use chrono_tz::Europe::Rome;
use crate::simulation_controller::gui::MyApp;

//Function to log events/commands from drones, clients and server.
pub fn logs(app: &mut MyApp, event: Event) {
    let current_time: DateTime<Utc> = Utc::now(); //Get current time.
    let local_time = current_time.with_timezone(&Rome); //Convert to Italian time.
    let formatted_time = local_time.format("%d-%m-%y %H:%M:%S").to_string(); //Format as string.

    let message = match event {
        Event::Drone(drone_event) => match drone_event {
            DroneEvent::PacketSent(packet) => {
                format!("[EVENT] Packet Sent to Node {}.",
                        packet
                            .routing_header
                            .hops
                            .get(packet.routing_header.hop_index)
                            .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                            .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            DroneEvent::PacketDropped(packet) => {
                format!("[EVENT] Packet Dropped to Node {}",
                        packet
                            .routing_header
                            .hops
                            .get(packet.routing_header.hop_index)
                            .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                            .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            DroneEvent::ControllerShortcut(packet) => {
                format!("[EVENT] Packet Routed through Controller by Node {}.",
                        packet
                            .routing_header
                            .hops
                            .get(packet.routing_header.hop_index)
                            .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                            .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
        },

        Event::Client(client_event) => match client_event {
            ClientEvent::PacketSent(packet) => {
                format!("[EVENT] Packet Sent to Client {}",
                        packet
                            .routing_header
                            .hops
                            .get(packet.routing_header.hop_index)
                            .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                            .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            ClientEvent::PacketReceived(packet) => {
                format!("[EVENT] Packet Received by Client: {}.", packet
                    .routing_header
                    .hops
                    .get(packet.routing_header.hop_index)
                    .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                    .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
        },

        Event::Server(server_event) => match server_event {
            ServerEvent::PacketSent(packet) => {
                format!("[EVENT] Packet Sent by Server: {}.", packet
                    .routing_header
                    .hops
                    .get(packet.routing_header.hop_index)
                    .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                    .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            ServerEvent::PacketReceived(packet) => {
                format!("[EVENT] Packet Received by Server: {}.", packet
                    .routing_header
                    .hops
                    .get(packet.routing_header.hop_index)
                    .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                    .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
        },
    };

    //Add log entry
    app.logs_vec.push(LogEntry {
        timestamp: formatted_time,
        message,
    });
}

pub fn filtered_logs(app: &mut MyApp) -> Vec<&LogEntry> {
    app.logs_vec.iter()
        .filter(|log| {
            // Filter by log type
            let matches_type = match log.message.split_whitespace().next() {
                Some("[EVENT]") => app.log_filters.show_events,
                Some("[COMMAND]") => app.log_filters.show_commands,
                _ => true,
            };

            // Filter by component
            let matches_component = if log.message.contains("Drone") {
                app.log_filters.show_drones
            } else if log.message.contains("Client") {
                app.log_filters.show_clients
            } else if log.message.contains("Server") {
                app.log_filters.show_servers
            } else {
                true
            };

            // Filter by search text
            let matches_search = app.log_filters.search_text.is_empty() ||
                log.message.to_lowercase().contains(&app.log_filters.search_text.to_lowercase()) ||
                log.timestamp.to_lowercase().contains(&app.log_filters.search_text.to_lowercase());

            matches_type && matches_component && matches_search
        })
        .collect()
}