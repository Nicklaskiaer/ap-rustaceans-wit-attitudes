use wg_2024::controller::DroneEvent;

use crate::simulation_controller::gui_structs::*;

use crate::client_server::network_core::{ClientEvent, ServerEvent};
use crate::simulation_controller::gui::MyApp;
use chrono::{DateTime, Utc};
use chrono_tz::Europe::Rome;

//Function to log events/commands from drones, clients and server.
pub fn logs(app: &mut MyApp, event: Event) {
    let current_time: DateTime<Utc> = Utc::now(); //Get current time.
    let local_time = current_time.with_timezone(&Rome); //Convert to Italian time.
    let formatted_time = local_time.format("%d-%m-%y %H:%M:%S").to_string(); //Format as string.

    let message = match event {
        Event::Drone(drone_event) => match drone_event {
            DroneEvent::PacketSent(packet) => {
                format!(
                    "[PACKET] Sent to Drone {}.",
                    packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            DroneEvent::PacketDropped(packet) => {
                format!(
                    "[PACKET] Dropped by Drone {}",
                    packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            DroneEvent::ControllerShortcut(packet) => {
                format!(
                    "[PACKET] Routed through Controller by Drone {}.",
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
                format!(
                    "[PACKET] Sent by Client {}",
                    packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            ClientEvent::PacketReceived(packet) => {
                format!(
                    "[PACKET] Received by Client: {}.",
                    packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            ClientEvent::MessageSent { from, to, content } => {
                format!("[MESSAGE] Sent by Client: {} to {}, content: {:?}", from, to, content)
            }
            ClientEvent::MessageReceived { receiver, content } => {
                format!("[MESSAGE] Received by Client: {}, content: {:?}", receiver, content)
            }
            ClientEvent::BrokenDroneDetected(drone_id) => {
                format!("[MESSAGE] Found Broken Drone, id: {}", drone_id)
            }
        },

        Event::Server(server_event) => match server_event {
            ServerEvent::PacketSent(packet) => {
                format!(
                    "[PACKET] Sent by Server: {}",
                    packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            ServerEvent::PacketReceived(packet) => {
                format!(
                    "[PACKET] Received by Server: {}",
                    packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists.
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case.
                )
            }
            ServerEvent::MessageSent { from, to, content } => {
                format!("[MESSAGE] Sent by Server: {} to {}, content: {:?}", from, to, content)
            }
            ServerEvent::MessageReceived { receiver, content } => {
                format!("[MESSAGE] Received by Server: {}, content: {:?}", receiver, content)
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
    app.logs_vec
        .iter()
        .filter(|log| {
            // Filter by log type
            let matches_type = match log.message.split_whitespace().next() {
                Some("[PACKET]") => app.log_filters.show_packet_events,
                Some("[MESSAGE]") => app.log_filters.show_command_events,
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
            let matches_search = app.log_filters.search_text.is_empty()
                || log
                    .message
                    .to_lowercase()
                    .contains(&app.log_filters.search_text.to_lowercase())
                || log
                    .timestamp
                    .to_lowercase()
                    .contains(&app.log_filters.search_text.to_lowercase());

            matches_type && matches_component && matches_search
        })
        .collect()
}
