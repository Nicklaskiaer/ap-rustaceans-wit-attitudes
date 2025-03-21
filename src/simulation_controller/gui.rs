use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Fragment, Packet};
use wg_2024::controller::{DroneCommand, DroneEvent};

use crate::simulation_controller::simulation_controller::SimulationController;
use crate::simulation_controller::gui_structs::*;
use crate::client::client::ClientEvent;
use crate::server::server::ServerEvent;

use eframe::egui;
use crossbeam_channel::{Receiver, Sender};
use chrono::{DateTime, Local, Utc};
use chrono_tz::Europe::Rome;
use std::collections::HashMap;

pub struct MyApp {
    simulation_controller: SimulationController,
    current_screen: Screen,
    logs_vec: Vec<LogEntry>,
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
    open_popups: HashMap<String, bool>,
    drone_packet_drop_rates: HashMap<String, String>,
    sender_to_rem: String,
    sender_to_add: String,
    topology: NetworkTopology,
    client_texture: Option<egui::TextureHandle>,
    server_texture: Option<egui::TextureHandle>,
    drone_texture: Option<egui::TextureHandle>,
}

pub struct NetworkTopology {
    pub nodes: Vec<Node>,
    pub connections: Vec<(usize, usize)>,
}

fn load_image(path: &str) -> Result<egui::ColorImage, image::ImageError> {
    let image_bytes = std::fs::read(path)?;
    let image = image::load_from_memory(&image_bytes)?;
    let size = [image.width() as usize, image.height() as usize];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}

impl MyApp {
    pub fn new(sc: SimulationController) -> MyApp {
        Self {
            simulation_controller: sc,
            current_screen: Screen::NetworkScreen,
            logs_vec: Vec::new(),
            show_confirmation_dialog: false,
            allowed_to_close: false,
            open_popups: HashMap::new(),
            drone_packet_drop_rates: HashMap::new(),
            sender_to_rem: String::new(),
            sender_to_add: String::new(),
            topology: NetworkTopology::new(),
            client_texture: None,
            server_texture: None,
            drone_texture: None,
        }
    }

    //Function to log events/commands from drones, clients and server.
    fn logs(&mut self, event: Event) {
        let current_time: DateTime<Utc> = Utc::now();
        let local_time = current_time.with_timezone(&Rome);
        let formatted_time = local_time.format("%d-%m-&Y %H:%M:%S").to_string();

        let message = match event {
            Event::Drone(drone_event) => match drone_event {
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
                    format!("[EVENT] Packet Routed through Controller by Node {}.",
                            packet
                                .routing_header
                                .hops
                                .get(packet.routing_header.hop_index)
                                .map(|&hop| hop.to_string()) // Convert u8 to String if it exists
                                .unwrap_or_else(|| "None".to_string()) // Handle the None case
                    )
                }
            },

            Event::Client(client_event) => match client_event {
                ClientEvent::PacketSent(packet) => {
                    format!("[EVENT] Packet Sent by Client {}",
                            packet
                                .routing_header
                                .hops
                                .get(packet.routing_header.hop_index)
                                .map(|&hop| hop.to_string()) // Convert u8 to String if it exists
                                .unwrap_or_else(|| "None".to_string()) // Handle the None case
                    )
                }
                ClientEvent::PacketReceived(packet) => {
                    format!("[EVENT] Packet Received by Client: {}.", packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case
                    )
                }
            },

            Event::Server(server_event) => match server_event {
                ServerEvent::PacketSent(packet) => {
                    format!("[EVENT] Packet Sent by Server: {}.", packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case
                    )
                }
                ServerEvent::PacketReceived(packet) => {
                    format!("[EVENT] Packet Received by Server: {}.", packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index)
                        .map(|&hop| hop.to_string()) // Convert u8 to String if it exists
                        .unwrap_or_else(|| "None".to_string()) // Handle the None case
                    )
                }
            },
        };

        //Add log entry
        self.logs_vec.push(LogEntry {
            timestamp: formatted_time,
            message,
        });
    }

    fn show_popup(&mut self, ctx: &egui::Context, name: &str) {
        let current_time: DateTime<Utc> = Utc::now(); // Get current time
        let italian_time = current_time.with_timezone(&Rome); // Convert to Italian time
        let formatted_time = italian_time.format("%d-%m-%Y %H:%M:%S").to_string(); // Format as string

        if let Some(is_open) = self.open_popups.get_mut(name) {
            egui::Window::new(format!("Controls for {}", name)).resizable(true).collapsible(true).open(is_open)
                .show(ctx, |ui| {
                    // Extract the node ID from the name
                    if let Some(node_id_str) = name.split_whitespace().nth(1) {
                        if let Ok(node_id) = node_id_str.parse::<NodeId>() {

                            // Handle Drone controls
                            if name.starts_with("Drone") {
                                if let Some((sender, neighbours)) = self.simulation_controller.get_drones().get(&node_id) {
                                    // Set Packet Drop Rate
                                    ui.horizontal(|ui| {
                                        ui.label("Packet Drop Rate:");
                                        let packet_drop_rate = self.drone_packet_drop_rates.entry(name.to_string()).or_insert_with(String::new);
                                        ui.text_edit_singleline(packet_drop_rate);

                                        if ui.button("Set").clicked() {
                                            if let Ok(value) = packet_drop_rate.parse::<f32>() {
                                                if value >= 0.0 && value <= 1.0 {
                                                    let message = format!("[COMMAND] Setting packet drop rate for {} to {}", name, value);
                                                    self.logs_vec.push(LogEntry {
                                                        timestamp: formatted_time.clone(),
                                                        message,
                                                    });
                                                    self.simulation_controller.handle_set_packet_drop_rate(node_id, value);
                                                }
                                            }
                                        }
                                    });

                                    // Add sender
                                    ui.horizontal(|ui| {
                                        ui.label("Add Sender:");
                                        ui.text_edit_singleline(&mut self.sender_to_add);

                                        if ui.button("Add").clicked() {
                                            match self.sender_to_add.parse::<NodeId>() {
                                                Ok(node_id) => {
                                                    //self.simulation_controller.handle_add_sender(node_id); todo

                                                    let message = format!("[COMMAND] Added sender {} to {}", node_id, name);
                                                    self.logs_vec.push(LogEntry {
                                                        timestamp: formatted_time.clone(),
                                                        message,
                                                    });
                                                }
                                                Err(_) => println!("Invalid input! Please enter a valid NodeId."),
                                            }
                                        }
                                    });

                                    // Remove sender
                                    ui.horizontal(|ui| {
                                        ui.label("Remove sender:");
                                        ui.text_edit_singleline(&mut self.sender_to_rem);

                                        if ui.button("Remove").clicked() {
                                            match self.sender_to_rem.parse::<NodeId>() {
                                                Ok(node_id) => {
                                                    //self.simulation_controller.handle_remove_sender(node_id);

                                                    let message = format!("[COMMAND] Removed sender {} to {}", node_id, name);
                                                    self.logs_vec.push(LogEntry {
                                                        timestamp: formatted_time.clone(),
                                                        message,
                                                    });
                                                },
                                                Err(_) => println!("Invalid input! Please enter a valid NodeId."),
                                            }
                                        }
                                    });

                                    // Crash Button
                                    if ui.button("Crash").clicked() {
                                        if let Some((_, neighbors)) = self.simulation_controller.get_drones().get(&node_id) {
                                            let message = format!("[COMMAND] Crashing {}", name);
                                            self.logs_vec.push(LogEntry {
                                                timestamp: formatted_time.clone(),
                                                message,
                                            });
                                            self.simulation_controller.handle_crash(node_id, neighbors.clone());
                                        }
                                    }
                                }
                            }

                            // Handle Client controls
                            else if name.starts_with("Client") {
                                ui.label("Client controls coming soon...");
                            }

                            // Handle Server controls
                            else if name.starts_with("Server") {
                                ui.label("Server controls coming soon...");
                            }
                        }
                    }
                });
        }
    }

}

impl eframe::App for MyApp{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for new events and log them
        while let Ok(event) = self.simulation_controller.get_drone_event_recv().try_recv(){
            match event {
                DroneEvent::PacketSent(_) => {println!("drone PacketSent")}
                DroneEvent::PacketDropped(_) => {println!("drone PacketDropped")}
                DroneEvent::ControllerShortcut(_) => {println!("drone ControllerShortcut")}
            }
            self.logs(Event::Drone(event));
        }

        while let Ok(event) = self.simulation_controller.get_client_event_recv().try_recv(){
            match event {
                ClientEvent::PacketSent(_) => {println!("client PacketSent")}
                ClientEvent::PacketReceived(_) => {println!("client PacketReceived")}
            }
            self.logs(Event::Client(event));
        }

        while let Ok(event) = self.simulation_controller.get_server_event_recv().try_recv(){
            match event {
                ServerEvent::PacketSent(_) => {println!("server PacketSent")}
                ServerEvent::PacketReceived(_) => {println!("server PacketReceived")}
            }
            self.logs(Event::Server(event));
        }

        if self.client_texture.is_none() {
            if let Ok(image) = load_image("images/client.png") {
                self.client_texture = Some(ctx.load_texture(
                    "client",
                    image,
                    Default::default()
                ));
            }
        }

        if self.server_texture.is_none() {
            if let Ok(image) = load_image("images/server.png") {
                self.server_texture = Some(ctx.load_texture(
                    "server",
                    image,
                    Default::default()
                ));
            }
        }

        if self.drone_texture.is_none() {
            if let Ok(image) = load_image("images/drone.png") {
                self.drone_texture = Some(ctx.load_texture(
                    "drone",
                    image,
                    Default::default()
                ));
            }
        }

        let current_drone_ids: Vec<String> = self.simulation_controller.get_drone_ids();
        self.open_popups.retain(|name, _| {
            let is_drone = name.starts_with("Drone");
            let is_client = name.starts_with("Client");
            let is_server = name.starts_with("Server");

            !is_drone || current_drone_ids.contains(name)
        });

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.allowed_to_close {
                // do nothing
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_confirmation_dialog = true;
            }
        }

        if self.show_confirmation_dialog{
            let screen_rect = ctx.screen_rect();
            let center_x = (screen_rect.left() + screen_rect.right()) / 2.0;
            let center_y = (screen_rect.top() + screen_rect.bottom()) / 2.0;

            let window_size = egui::vec2(170.0, 150.0);

            let top_left = egui::pos2(center_x - window_size.x / 2.0, center_y - window_size.y / 2.0);

            egui::Window::new("Confirm Exit").fixed_size(window_size).fixed_pos(top_left).collapsible(false).resizable(false)
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
        }else{
            egui::TopBottomPanel::top("navigation_panel").show(ctx, |ui| {
                ui.add_space(3.0);

                ui.horizontal(|ui| {
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
                        egui::SidePanel::left("network_menu").min_width(140.0).max_width(140.0)
                            .show(ctx, |ui| {
                                ui.heading("Network Menu");

                                ui.separator();
                                ui.label("Clients:");
                                for client in &self.simulation_controller.get_client_ids() {
                                    if ui.button(client).clicked() {
                                        self.open_popups.insert(client.clone(), true);
                                    }
                                }

                                ui.separator();
                                ui.label("Servers:");
                                for server in &self.simulation_controller.get_server_ids() {
                                    if ui.button(server).clicked() {
                                        self.open_popups.insert(server.clone(), true);
                                    }
                                }

                                ui.separator();
                                ui.label("Drones:");
                                for drones in &self.simulation_controller.get_drone_ids() {
                                    if ui.button(drones).clicked() {
                                        self.open_popups.insert(drones.clone(), true);
                                    }
                                }
                            });

                        egui::CentralPanel::default().show(ctx, |ui| {
                            self.topology.draw(
                                ui,
                                self.client_texture.as_ref(),
                                self.server_texture.as_ref(),
                                self.drone_texture.as_ref()
                            );
                        });
                    },

                    Screen::LogsScreen => {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for log in &self.logs_vec {
                                    let mut text_parts: Vec<egui::RichText> = Vec::new();

                                    if log.message.starts_with("[EVENT]") {
                                        text_parts.push(egui::RichText::new("[EVENT]").color(egui::Color32::GREEN));
                                        text_parts.push(egui::RichText::new(&log.message[7..]).color(egui::Color32::WHITE));
                                    } else if log.message.starts_with("[COMMAND]") {
                                        text_parts.push(egui::RichText::new("[COMMAND]").color(egui::Color32::BLUE));
                                        text_parts.push(egui::RichText::new(&log.message[9..]).color(egui::Color32::WHITE));
                                    }

                                    let formatted_log = egui::RichText::new(format!("[{}] ", log.timestamp)).color(egui::Color32::WHITE);

                                    // Combine all parts and display the log
                                    ui.horizontal(|ui| {
                                        ui.label(formatted_log);
                                        for part in text_parts {
                                            ui.label(part);
                                        }
                                    });
                                }
                            });
                        });
                    }
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

            if self.current_screen == Screen::NetworkScreen {

                self.topology.update_topology(
                    &self.simulation_controller.get_drones(),
                    &self.simulation_controller.get_clients(),
                    &self.simulation_controller.get_servers()
                );

                let legend_width = 150.0;
                let legend_height = 100.0;

                egui::Window::new("Legend").anchor(egui::Align2::RIGHT_TOP, [-10.0, 40.0]).collapsible(false).resizable(false).default_width(legend_width).default_height(legend_height)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| { ui.colored_label(egui::Color32::BLUE, " ● Drone"); });
                        ui.horizontal(|ui| { ui.colored_label(egui::Color32::RED, " ● Client"); });
                        ui.horizontal(|ui| { ui.colored_label(egui::Color32::GREEN, " ● Server"); });
                    });
            }
        }
    }
}

impl NetworkTopology {
    pub fn new() -> Self {
        NetworkTopology {
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn update_topology(
        &mut self,
        drones: &HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)>,
        clients: &HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)>,
        servers: &HashMap<NodeId, Vec<NodeId>>
    ) {
        self.nodes.clear();
        self.connections.clear();

        let n = drones.len();
        let center = (300.0, 300.0);
        let radius = 100.0;
        let offset = 50.0;
        let client_offset_x = -20.0; // Move clients slightly left
        let server_offset_x = 20.0;  // Move servers slightly right

        let angle_increment = std::f32::consts::TAU / n as f32;
        let mut node_positions = HashMap::new();

        //Assign positions to drones
        for (i, (node_id, _)) in drones.iter().enumerate() {
            let angle = i as f32 * angle_increment;
            let x = center.0 + radius * angle.cos();
            let y = center.1 + radius * angle.sin();

            node_positions.insert(*node_id, (x, y));

            self.nodes.push(Node {
                id: node_id.to_string(),
                position: (x, y),
                is_client: false,
                is_server: false,
            });
        }

        // Assign positions to clients
        for (client_id, (_, neighbors)) in clients {
            if let Some(neighbor_id) = neighbors.first() {
                if let Some(&(dx, dy)) = node_positions.get(neighbor_id) {
                    let direction = ((dx - center.0), (dy - center.1));
                    let norm = (direction.0.powi(2) + direction.1.powi(2)).sqrt();

                    if norm > 0.0 {
                        let scale = (radius + offset * 2.0) / norm;
                        let x = center.0 + direction.0 * scale + client_offset_x; // Move client left
                        let y = center.1 + direction.1 * scale;

                        node_positions.insert(*client_id, (x, y));

                        self.nodes.push(Node {
                            id: client_id.to_string(),
                            position: (x, y),
                            is_client: true,
                            is_server: false,
                        });
                    }
                }
            }
        }

        // Assign positions to servers
        for (server_id, neighbors) in servers {
            if let Some(neighbor_id) = neighbors.first() {
                if let Some(&(dx, dy)) = node_positions.get(neighbor_id) {
                    let direction = ((dx - center.0), (dy - center.1));
                    let norm = (direction.0.powi(2) + direction.1.powi(2)).sqrt();

                    if norm > 0.0 {
                        let scale = (radius + offset * 2.0) / norm;
                        let x = center.0 + direction.0 * scale + server_offset_x; // Move server right
                        let y = center.1 + direction.1 * scale;

                        node_positions.insert(*server_id, (x, y));

                        self.nodes.push(Node {
                            id: server_id.to_string(),
                            position: (x, y),
                            is_client: false,
                            is_server: true,
                        });
                    }
                }
            }
        }

        //Add connections
        for (node_id, (_, neighbors)) in drones.iter().chain(clients.iter()) {
            if let Some(_pos1) = node_positions.get(node_id) {
                for neighbor_id in neighbors {
                    if let Some(_pos2) = node_positions.get(neighbor_id) {
                        let idx1 = self.nodes.iter().position(|n| n.id == node_id.to_string()).unwrap();
                        let idx2 = self.nodes.iter().position(|n| n.id == neighbor_id.to_string()).unwrap();
                        self.connections.push((idx1, idx2));
                    }
                }
            }
        }
    }

    fn draw(
        &self,
        ui: &mut egui::Ui,
        client_tex: Option<&egui::TextureHandle>,
        server_tex: Option<&egui::TextureHandle>,
        drone_tex: Option<&egui::TextureHandle>,
    ) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());

        // **Draw connections**
        for &(node1_idx, node2_idx) in &self.connections {
            let node1 = &self.nodes[node1_idx];
            let node2 = &self.nodes[node2_idx];

            let pos1 = response.rect.min + egui::vec2(node1.position.0, node1.position.1);
            let pos2 = response.rect.min + egui::vec2(node2.position.0, node2.position.1);

            let color = if node1.is_client || node2.is_client {
                egui::Color32::RED
            } else if node1.is_server || node2.is_server {
                egui::Color32::GREEN
            } else {
                egui::Color32::LIGHT_GRAY
            };

            painter.line_segment([pos1, pos2], egui::Stroke::new(2.0, color));
        }

        // **Draw nodes**
        for node in &self.nodes {
            let pos = response.rect.min + egui::vec2(node.position.0, node.position.1);

            let texture = if node.is_client {
                client_tex
            } else if node.is_server {
                server_tex
            } else {
                drone_tex
            };

            let rect = egui::Rect::from_center_size(
                pos,
                egui::Vec2::new(40.0, 40.0),
            );

            if let Some(texture_handle) = texture {
                painter.image(
                    texture_handle.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            
            painter.text(
                pos,
                egui::Align2::CENTER_TOP,
                &node.id,
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
        }
    }
}