use chrono::{DateTime, Utc};
use chrono_tz::Europe::Rome;
use crossbeam_channel::Receiver;
use eframe::egui;
use std::collections::HashMap;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Fragment, Packet};
use crate::simulation_controller::simulation_controller::SimulationController;

#[derive(PartialEq)]
enum Screen {
    NetworkScreen,
    LogsScreen,
}

struct LogEntry {
    timestamp: String,
    message: String,
}

pub struct MyApp {
    current_screen: Screen,
    logs: Vec<LogEntry>, // List of logs
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
    node_event_recv: Receiver<DroneEvent>,
    clients: Vec<String>, // List of clients
    servers: Vec<String>, // List of servers
    drones: Vec<String>, // List of Drones
    open_popups: HashMap<String, bool>, // Keeps tracks of opened control windows
    drone_packet_drop_rates: HashMap<String, String>, // Keeps the input state for each drone
    simulation_controller: SimulationController,
}

impl MyApp {
    pub(crate) fn new(sc: SimulationController) -> Self {
        Self {
            current_screen: Screen::NetworkScreen,
            logs: Vec::new(),
            show_confirmation_dialog: false,
            allowed_to_close: false,
            node_event_recv: sc.get_node_event_recv(),
            clients: vec!["Test_Client1".to_string(), "Test_Client2".to_string()], // Example clients
            servers: vec!["Test_Server1".to_string(), "Test_Server1".to_string()], // Example servers
            drones: sc.get_drone_ids(),
            open_popups: HashMap::new(), // Initialize the map to track popups
            drone_packet_drop_rates: HashMap::new(),
            simulation_controller: sc,
        }
    }

    fn log_command(&mut self, command: DroneCommand) {
        let current_time: DateTime<Utc> = Utc::now(); // Get current time
        let italian_time = current_time.with_timezone(&Rome); // Convert to Italian time
        let formatted_time = italian_time.format("%d-%m-%Y %H:%M:%S").to_string(); // Format as string

        let message = match command {
            DroneCommand::RemoveSender(node_id) => {
                format!("[COMMAND] Removed Sender at Node {}", node_id)
            }
            DroneCommand::AddSender(node_id, _) => {
                format!("[COMMAND] Added Sender at Node {}", node_id)
            }
            DroneCommand::SetPacketDropRate(_) => {
                return; //todo(how to get ID of changed drop rate drone)
            }
            DroneCommand::Crash => {
                return; //todo(how to get ID of crashed drone)
            }
        };

        // Add the log entry
        self.logs.push(LogEntry {
            timestamp: formatted_time,
            message,
        });
    }

    fn log_event(&mut self, event: DroneEvent) {
        let current_time: DateTime<Utc> = Utc::now(); // Get current time
        let italian_time = current_time.with_timezone(&Rome); // Convert to Italian time
        let formatted_time = italian_time.format("%d-%m-%Y %H:%M:%S").to_string(); // Format as string

        let message = match event {
            DroneEvent::PacketSent(packet) => {
                format!("[EVENT] Packet Sent by Node {}.",
                        packet
                            .routing_header
                            .hops
                            .get(packet.routing_header.hop_index)
                            .map(|&hop| hop.to_string()) // Convert u8 to String if it exists
                            .unwrap_or_else(|| "None".to_string()) // Handle the None case
                )
            }
            DroneEvent::PacketDropped(packet) => {
                format!("[EVENT] Packet Dropped by Node {}",
                        packet
                            .routing_header
                            .hops
                            .get(packet.routing_header.hop_index)
                            .map(|&hop| hop.to_string()) // Convert u8 to String if it exists
                            .unwrap_or_else(|| "None".to_string()) // Handle the None case
                )
            }
            DroneEvent::ControllerShortcut(packet) => {
                format!("[EVENT] Packet Routed trough Controller by Node {}.",
                        packet
                            .routing_header
                            .hops
                            .get(packet.routing_header.hop_index)
                            .map(|&hop| hop.to_string()) // Convert u8 to String if it exists
                            .unwrap_or_else(|| "None".to_string()) // Handle the None case
                )
            }
        };

        // Add the log entry
        self.logs.push(LogEntry {
            timestamp: formatted_time,
            message,
        });
    }

    fn show_popup(&mut self, ctx: &egui::Context, name: &str) {
        if let Some(is_open) = self.open_popups.get_mut(name) {
            egui::Window::new(format!("Controls for {}", name))
                .resizable(true)
                .collapsible(true)
                .open(is_open) // Tie window open state to the hashmap
                .show(ctx, |ui| {
                    if self.clients.contains(&name.to_string()) {
                        //todo!("implement controls for client)

                    } else if self.drones.contains(&name.to_string()) {

                        // Get or initialize the input value for the packet drop rate
                        let packet_drop_rate = self
                            .drone_packet_drop_rates
                            .entry(name.to_string())
                            .or_insert_with(|| String::new());

                        // Packet Drop Rate Control
                        ui.horizontal(|ui| {
                            ui.label("Packet Drop Rate:");
                            ui.text_edit_singleline(packet_drop_rate); // Packet drop rate input field
                            if ui.button("Set").clicked() {
                                match packet_drop_rate.parse::<f32>() {
                                    Ok(value) if value >= 0.0 && value <= 1.0 => {
                                        println!("Setting packet drop rate for {} to {}", name, value);
                                        //self.handle_set_packet_drop_rate(xyz, value); todo!("handle the packet drop rate control")
                                    }
                                    _ => {
                                        println!("Invalid drop rate value. Please enter a number between 0 and 1.");
                                    }
                                }
                            }
                        });

                        if ui.button("Crash").clicked() {
                            println!("Crashed {}.", name); //todo!("handle crash")
                        }
                    } else if self.servers.contains(&name.to_string()) {
                        //todo!("implement controls for client)
                    }
                });
        }
    }

}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for new events and log them
        while let Ok(event) = self.node_event_recv.try_recv(){
            println!("\n\n\n\n\n\n\n\n\n\n\n\naaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\n\n\n\n\n\n\n\n\n\n\n");
            self.log_event(event); // Use the new log_event method
        }

        egui::TopBottomPanel::top("navigation_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Navigation buttons
                if ui.button("Network Topology").clicked() {
                    self.current_screen = Screen::NetworkScreen;
                }

                if ui.button("Logs Page").clicked() {
                    self.current_screen = Screen::LogsScreen;
                }
            });
        });

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.allowed_to_close {
                // do nothing - we will close
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_confirmation_dialog = true;
            }
        }

        if self.show_confirmation_dialog {
            egui::Window::new("Do you want to quit?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("No").clicked() {
                            self.show_confirmation_dialog = false;
                            self.allowed_to_close = false;
                        }

                        if ui.button("Yes").clicked() {
                            self.show_confirmation_dialog = false;
                            self.allowed_to_close = true;
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_screen {
                Screen::NetworkScreen => {
                    egui::SidePanel::left("network_menu")
                        .min_width(200.0) // Set minimum width
                        .max_width(200.0)
                        .show(ctx, |ui| {
                            ui.heading("Network Menu");

                            ui.separator();
                            ui.label("Clients:");
                            for client in &self.clients {
                                if ui.button(client).clicked() {
                                    // Explicitly set the pop-up state to true to reopen it
                                    self.open_popups.insert(client.clone(), true);
                                }
                            }

                            ui.separator();
                            ui.label("Servers:");
                            for server in &self.servers {
                                if ui.button(server).clicked() {
                                    // Explicitly set the pop-up state to true to reopen it
                                    self.open_popups.insert(server.clone(), true);
                                }
                            }

                            ui.separator();
                            ui.label("Drones:");
                            for drones in &self.drones {
                                if ui.button(drones).clicked() {
                                    // Explicitly set the pop-up state to true to reopen it
                                    self.open_popups.insert(drones.clone(), true);
                                }
                            }
                        });

                },
                Screen::LogsScreen => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for log in &self.logs {
                            // Format the log entry as [timestamp] event_type: message
                            let formatted_log = format!("{} | {}", log.timestamp, log.message);
                            ui.label(formatted_log); // Display the formatted log entry
                        }
                    });
                },
            }
        });

        let popups_to_show: Vec<String> = self
            .open_popups
            .iter()
            .filter(|(_, &open)| open)
            .map(|(name, _)| name.clone())
            .collect();

        // Show pop-ups
        for name in popups_to_show {
            self.show_popup(ctx, &name);
        }

        // #[cfg(test)]
        // {
        //     const TIMEOUT: Duration = Duration::from_millis(400);
        // 
        //     let msg = Packet::new_fragment(
        //         SourceRoutingHeader {
        //             hop_index: 1,
        //             hops: vec![1, 11, 12, 21],
        //         },
        //         1,
        //         Fragment {
        //             fragment_index: 1,
        //             total_n_fragments: 1,
        //             length: 128,
        //             data: [1; 128],
        //         },
        //     );
        // 
        //     // Get the sender for drone 11 from packet_channels
        //     let d11_send = &packet_channels[&11].0;
        // 
        //     //D12 sends packet to D11
        //     d11_send.send(msg.clone()).unwrap();
        // }
    }
}