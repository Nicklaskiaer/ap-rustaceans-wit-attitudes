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

struct Node {
    id: String,
    position: (f32, f32), // x, y position in the graph
}

pub struct NetworkTopology {
    pub nodes: Vec<Node>,
    pub connections: Vec<(usize, usize)>, // Edges: indices of nodes
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
    topology: NetworkTopology,
}

impl MyApp {
    pub(crate) fn new(sc: SimulationController) -> Self {
        Self {
            current_screen: Screen::NetworkScreen,
            logs: Vec::new(),
            show_confirmation_dialog: false,
            allowed_to_close: false,
            node_event_recv: sc.get_drone_event_recv(),
            clients: vec!["Test_Client1".to_string(), "Test_Client2".to_string()], // Example clients
            servers: vec!["Test_Server1".to_string(), "Test_Server1".to_string()], // Example servers
            drones: sc.get_drone_ids(),
            open_popups: HashMap::new(), // Initialize the map to track popups
            drone_packet_drop_rates: HashMap::new(),
            simulation_controller: sc,
            topology: NetworkTopology::new(),
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
                        //todo!(implement controls for client)

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
                                        //self.handle_set_packet_drop_rate(name, value);
                                    }
                                    _ => {
                                        println!("Invalid drop rate value. Please enter a number between 0 and 1.");
                                    }
                                }
                            }
                        });

                        if ui.button("Crash").clicked() {
                            println!("Crashed {}.", name);
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

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.allowed_to_close {
                // do nothing - we will close
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_confirmation_dialog = true;
            }
        }

        if self.show_confirmation_dialog {
            let screen_rect = ctx.screen_rect();
            let center_x = (screen_rect.left() + screen_rect.right()) / 2.0;
            let center_y = (screen_rect.top() + screen_rect.bottom()) / 2.0;

            // Set the size of the confirmation dialog
            let window_size = egui::vec2(170.0, 150.0);

            // Calculate top-left position for the window to be centered
            let top_left = egui::pos2(center_x - window_size.x / 2.0, center_y - window_size.y / 2.0);

            egui::Window::new("Confirm Exit")
                .fixed_size(window_size) // Fix the size of the dialog
                .fixed_pos(top_left)    // Position it at the calculated top-left point
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                            ui.label("Are you sure you want to exit?");
                            ui.separator();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            let button = egui::Button::new("Exit").fill(egui::Color32::from_rgb(0, 0, 250)); // Red fill color
                            if ui.add(button).clicked() {
                                self.show_confirmation_dialog = false;
                                self.allowed_to_close = true;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }

                            if ui.button("Cancel").clicked() {
                                self.show_confirmation_dialog = false;
                            }
                        });
                    });
                });
        }

        if !self.show_confirmation_dialog {
            egui::TopBottomPanel::top("navigation_panel").show(ctx, |ui| {
                ui.add_space(3.0);

                ui.horizontal(|ui| {
                    // Navigation buttons
                    if ui.button("Network Topology").clicked() {
                        self.current_screen = Screen::NetworkScreen;
                    }

                    if ui.button("Logs Page").clicked() {
                        self.current_screen = Screen::LogsScreen;
                    }
                });

                ui.add_space(2.0)
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                match self.current_screen {
                    Screen::NetworkScreen => {
                        // Synchronize the drones with the topology
                        self.topology.update_drones(&self.drones);

                        egui::SidePanel::left("network_menu")
                            .min_width(160.0) // Set minimum width
                            .max_width(160.0)
                            .show(ctx, |ui| {
                                ui.heading("Network Menu");

                                ui.separator();
                                ui.label("Clients:");
                                for client in &self.clients {
                                    if ui.button(client).clicked() {
                                        // Set the pop-up state to true to reopen it
                                        self.open_popups.insert(client.clone(), true);
                                    }
                                }

                                ui.separator();
                                ui.label("Servers:");
                                for server in &self.servers {
                                    if ui.button(server).clicked() {
                                        // Set the pop-up state to true to reopen it
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

                        egui::CentralPanel::default().show(ctx, |ui| {
                            self.topology.draw(ui);
                        });
                    },
                    Screen::LogsScreen => {
                        egui::SidePanel::left("logs_options")
                            .min_width(160.0) // Set minimum width
                            .max_width(160.0)
                            .show(ctx, |ui| {
                                ui.heading("Logs Options");
                                ui.separator();

                                ui.label("Sort by: ");
                            });

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for log in &self.logs {
                                let mut text_parts: Vec<egui::RichText> = Vec::new();

                                if log.message.starts_with("[EVENT]") {
                                    text_parts.push(egui::RichText::new("[EVENT]").color(egui::Color32::GREEN));
                                    text_parts.push(egui::RichText::new(&log.message[7..]).color(egui::Color32::WHITE)); // Rest of the message
                                } else if log.message.starts_with("[COMMAND]") {
                                    text_parts.push(egui::RichText::new("[COMMAND]").color(egui::Color32::BLUE));
                                    text_parts.push(egui::RichText::new(&log.message[9..]).color(egui::Color32::WHITE)); // Rest of the message
                                }

                                // Add timestamp in white
                                let formatted_log = egui::RichText::new(format!("{} | ", log.timestamp)).color(egui::Color32::WHITE);

                                // Combine all parts and display the log
                                ui.horizontal(|ui| {
                                    ui.label(formatted_log); // Timestamp
                                    for part in text_parts {
                                        ui.label(part); // Colored and white components
                                    }
                                });
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

        }

        /*#[cfg(test)]
         {
             const TIMEOUT: Duration = Duration::from_millis(400);

             let msg = Packet::new_fragment(
                 SourceRoutingHeader {
                     hop_index: 1,
                     hops: vec![1, 11, 12, 21],
                 },
                 1,
                 Fragment {
                     fragment_index: 1,
                     total_n_fragments: 1,
                     length: 128,
                     data: [1; 128],
                 },
             );

             // Get the sender for drone 11 from packet_channels
             let d11_send = &packet_channels[&11].0;

             //D12 sends packet to D11
             d11_send.send(msg.clone()).unwrap();
         }*/
    }
}

impl NetworkTopology {
    pub fn new() -> Self {
        NetworkTopology {
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn update_drones(&mut self, drones: &[String]) {
        let mut x = 100.0;
        // Clear existing nodes and connections
        self.nodes.clear();
        self.connections.clear();

        // Add drones as nodes
        for (i, drone) in drones.iter().enumerate() {
            x = x+50.0;
            self.nodes.push(Node {
                id: drone.clone(),
                position: (200.0 + i as f32 * 100.0, x), // Dynamic position based on index
            });
        }

        for i in 0..self.nodes.len() {
            self.connections.push((0, i)); // Connect all drones to the first node (central hub)
        }
    }

    fn draw(&self, ui: &mut egui::Ui) {
        // Create a painter constrained to the available area
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());

        // Draw connections (edges) first
        for &(node1_idx, node2_idx) in &self.connections {
            let pos1 = self.nodes[node1_idx].position;
            let pos2 = self.nodes[node2_idx].position;

            // Calculate positions relative to the panel
            let p1 = response.rect.min + egui::vec2(pos1.0, pos1.1);
            let p2 = response.rect.min + egui::vec2(pos2.0, pos2.1);

            // Draw the line for the edge
            painter.line_segment([p1, p2], egui::Stroke::new(2.0, egui::Color32::LIGHT_GRAY));
        }

        // Draw nodes (circles with labels)
        for node in &self.nodes {
            let pos = response.rect.min + egui::vec2(node.position.0, node.position.1);

            // Draw the circle for the node
            painter.circle_filled(pos, 10.0, egui::Color32::BLUE);

            // Add a label for the node
            painter.text(
                pos + egui::vec2(0.0, -20.0),
                egui::Align2::CENTER_CENTER,
                &node.id,
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
        }
    }
}