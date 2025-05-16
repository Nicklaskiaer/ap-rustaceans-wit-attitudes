use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Fragment, Packet};
use wg_2024::controller::{DroneCommand, DroneEvent};

use crate::simulation_controller::simulation_controller::SimulationController;
use crate::simulation_controller::gui_structs::*;
use crate::client::client::ClientEvent;
use crate::server::server::{ServerEvent, ServerType};
use crate::simulation_controller::popup_handler;
use crate::simulation_controller::logs_handler;

use eframe::egui;
use crossbeam_channel::{Receiver, Sender};
use chrono::{DateTime, Local, Utc};
use chrono_tz::Europe::Rome;
use std::collections::HashMap;
use std::time::Duration;
use crate::client::client_server_command::ClientServerCommand;
use crate::testing;

pub struct MyApp {
    pub(crate) simulation_controller: SimulationController,
    current_screen: Screen, //Network diagram or Logs Page screen.
    pub(crate) logs_vec: Vec<LogEntry>, //Vector of logs shown in the Logs page
    show_confirmation_dialog: bool, //Confirmation dialog box when clicking "X" button of the window.
    allowed_to_close: bool, //Confirm closing the program window.
    pub(crate) open_popups: HashMap<String, bool>, //Hashmap of popup windows for clients and drones.
    pub(crate) slider_temp_pdrs: HashMap<NodeId, f32>, //Hashmap of displayed PDR's of drones.
    pub(crate) drone_text_inputs: HashMap<NodeId, String>, //Hashmap of inputs for drones (add/rem sender id).
    pub log_filters: LogFilters,
    pub client_message_inputs: HashMap<NodeId, String>,
    pub selected_server: HashMap<NodeId, String>,
    pub client_popup_screens: HashMap<NodeId, ClientPopupScreen>,
    topology: NetworkTopology,
    client_texture: Option<egui::TextureHandle>, //Icon for clients in diagram.
    server_texture: Option<egui::TextureHandle>, //Icon for servers in diagram.
    drone_texture: Option<egui::TextureHandle>,  //Icon for drones in diagram.
    topology_needs_update: bool,
}

pub struct NetworkTopology {
    pub nodes: Vec<Node>, //Vector of nodes (clients, servers and drones) in the network topology graph.
    pub connections: Vec<(usize, usize)>, //Connections (lines) between nodes.
}

fn load_image(path: &str) -> Result<egui::ColorImage, image::ImageError> {  //Function to load Icons of clients, server and drones.
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
            slider_temp_pdrs: HashMap::new(),
            drone_text_inputs: HashMap::new(),
            log_filters: LogFilters::default(),
            client_message_inputs: HashMap::new(),
            selected_server: HashMap::new(),
            client_popup_screens: HashMap::new(),
            topology: NetworkTopology::new(),
            client_texture: None,
            server_texture: None,
            drone_texture: None,
            topology_needs_update: true,
        }
    }

    fn show_popup(&mut self, ctx: &egui::Context, name: &str) {
        popup_handler::show_popup(self, ctx, name);
    }

    fn logs(&mut self, event: Event){
        logs_handler::logs(self, event);
    }
}

impl eframe::App for MyApp{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //Poll for new events and log them.
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

        //Load icon textures for nodes in graph.
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

        /*let current_drone_ids: Vec<String> = self.simulation_controller.get_drone_ids();
        self.open_popups.retain(|name, _| {
            let is_drone = name.starts_with("Drone");
            let is_client = name.starts_with("Client");
            let is_server = name.starts_with("Server");

            !is_drone || current_drone_ids.contains(name)
        });*/

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.allowed_to_close {
                //dn.
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
                        egui::SidePanel::left("log_filters")
                            .min_width(140.0)
                            .max_width(140.0)
                            .show(ctx, |ui| {
                                ui.heading("Log Filters");
                                ui.separator();

                                ui.checkbox(&mut self.log_filters.show_events, "Show Events");
                                ui.checkbox(&mut self.log_filters.show_commands, "Show Commands");

                                ui.separator();

                                ui.checkbox(&mut self.log_filters.show_drones, "Show Drones");
                                ui.checkbox(&mut self.log_filters.show_clients, "Show Clients");
                                ui.checkbox(&mut self.log_filters.show_servers, "Show Servers");

                                ui.separator();

                                ui.label("Search:");
                                ui.text_edit_singleline(&mut self.log_filters.search_text);
                            });

                        egui::CentralPanel::default().show(ctx, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for log in logs_handler::filtered_logs(self) {
                                    let mut text_parts: Vec<egui::RichText> = Vec::new();

                                    if log.message.starts_with("[EVENT]") {
                                        text_parts.push(egui::RichText::new("[EVENT]").color(egui::Color32::GREEN));
                                        text_parts.push(egui::RichText::new(&log.message[7..]).color(egui::Color32::WHITE));
                                    } else if log.message.starts_with("[COMMAND]") {
                                        text_parts.push(egui::RichText::new("[COMMAND]").color(egui::Color32::BLUE));
                                        text_parts.push(egui::RichText::new(&log.message[9..]).color(egui::Color32::WHITE));
                                    }

                                    let formatted_log = egui::RichText::new(format!("[{}] ", log.timestamp)).color(egui::Color32::WHITE);

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
                if self.current_screen == Screen::NetworkScreen && self.topology_needs_update {
                    self.topology.update_topology(
                        &self.simulation_controller.get_drones().iter()
                            .map(|(id, (sender, neighbors, _))| (*id, (sender.clone(), neighbors.clone())))
                            .collect::<HashMap<NodeId, (Sender<DroneCommand>, Vec<NodeId>)>>(),
                        &self.simulation_controller.get_clients(),
                        &self.simulation_controller.get_servers()
                    );
                    self.topology_needs_update = false;
                }
            
                let legend_width = 150.0;
                let legend_height = 100.0;
            
                egui::Window::new("Legend").anchor(egui::Align2::RIGHT_TOP, [-10.0, 40.0]).collapsible(false).resizable(false).default_width(legend_width).default_height(legend_height)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| { ui.colored_label(egui::Color32::WHITE, " ● Drone"); });
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
        clients: &HashMap<NodeId, (Sender<ClientServerCommand>, Vec<NodeId>)>,
        servers: &HashMap<NodeId, (Sender<ClientServerCommand>, Vec<NodeId>, ServerType)>
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
        for (server_id, (_, neighbors, _)) in servers {
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
        // for (node_id, (_, neighbors)) in drones.iter().chain(clients.iter()) {
        //     if let Some(_pos1) = node_positions.get(node_id) {
        //         for neighbor_id in neighbors {
        //             if let Some(_pos2) = node_positions.get(neighbor_id) {
        //                 let idx1 = self.nodes.iter().position(|n| n.id == node_id.to_string()).unwrap();
        //                 let idx2 = self.nodes.iter().position(|n| n.id == neighbor_id.to_string()).unwrap();
        //                 self.connections.push((idx1, idx2));
        //             }
        //         }
        //     }
        // }

        // Add connections for drones
        for (node_id, (_, neighbors)) in drones.iter() {
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

        // Add connections for clients
        for (node_id, (_, neighbors)) in clients.iter() {
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
        &mut self,
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
        for node in &mut self.nodes {
            let icon_size = egui::Vec2::new(50.0, 50.0);
            let center_pos = response.rect.min + egui::vec2(node.position.0, node.position.1);
            let icon_rect = egui::Rect::from_center_size(center_pos, icon_size);

            // Enable click + drag
            let interact = ui.interact(icon_rect, egui::Id::new(&node.id), egui::Sense::click_and_drag());

            // Handle dragging: update position based on mouse delta
            if interact.dragged() {
                let delta = interact.drag_delta();
                node.position.0 += delta.x;
                node.position.1 += delta.y;
            }

            // Choose correct texture
            let texture = if node.is_client {
                client_tex
            } else if node.is_server {
                server_tex
            } else {
                drone_tex
            };

            // Draw icon
            if let Some(texture_handle) = texture {
                ui.painter().image(
                    texture_handle.id(),
                    icon_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }

            // Label background + text
            let label_rect = egui::Rect::from_center_size(
                center_pos,
                egui::Vec2::new(32.0, 18.0),
            );
            ui.painter().rect_filled(label_rect, 4.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180));
            ui.painter().text(
                center_pos,
                egui::Align2::CENTER_CENTER,
                &node.id,
                egui::TextStyle::Monospace.resolve(ui.style()),
                egui::Color32::WHITE,
            );
        }

    }
}