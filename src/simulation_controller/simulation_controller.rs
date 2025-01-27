use std::collections::HashMap;
use chrono::{DateTime, Utc};
use chrono_tz::Europe::Rome;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use eframe::{egui};
use wg_2024::packet::Packet;

pub struct SimulationController {
    drones: HashMap<NodeId, Sender<DroneCommand>>,
    node_event_recv: Receiver<DroneEvent>,
    node_command_recv: Receiver<DroneCommand>,
}

impl SimulationController {
    pub fn new(drones: HashMap<NodeId, Sender<DroneCommand>>, node_event_recv: Receiver<DroneEvent>, node_command_recv: Receiver<DroneCommand>)->Self{
        SimulationController{
            drones,
            node_event_recv,
            node_command_recv
        }
    }

    pub fn crash(&mut self, drone_id: NodeId, neighbors: Vec<NodeId>) {
        if let Some(crashed_drone_sender) = self.drones.get(&drone_id) {
            // send crash command
            crashed_drone_sender.send(DroneCommand::Crash).unwrap();

            for neighbor in neighbors {
                if let Some(neighbor_drone_sender) = self.drones.get(&neighbor) {
                    // remove drone from neighbor
                    neighbor_drone_sender.send(DroneCommand::RemoveSender(drone_id)).unwrap();

                    // remove neighbor form drone
                    crashed_drone_sender.send(DroneCommand::RemoveSender(neighbor)).unwrap();
                }
            }
        }
    }

    pub fn handle_remove_sender(&self, drone_sender_id: NodeId, drone_id: NodeId) {
        if let Some(drone_sender) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::RemoveSender(drone_id)).unwrap();
        }
    }

    pub fn handle_add_sender(&self, drone_sender_id: NodeId, drone_id: NodeId, drone_packet: Sender<Packet>) {
        if let Some(drone_sender) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::AddSender(drone_id, drone_packet)).unwrap();
        }
    }

    pub fn handle_set_packet_drop_rate(&self, drone_sender_id: NodeId, drop_rate: f32) {
        if let Some(drone_sender) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::SetPacketDropRate(drop_rate)).unwrap();
        }
    }

    pub fn handle_crash(&self, drone_sender_id: NodeId, neighbors: Vec<NodeId>) {
        if let Some(crashed_drone_sender) = self.drones.get(&drone_sender_id) {
            // send crash command to the drone
            crashed_drone_sender.send(DroneCommand::Crash).unwrap();

            for neighbor in neighbors {
                if let Some(neighbor_drone_sender) = self.drones.get(&neighbor) {
                    // remove drone from neighbor
                    neighbor_drone_sender.send(DroneCommand::RemoveSender(drone_sender_id)).unwrap();

                    // remove neighbor form drone
                    crashed_drone_sender.send(DroneCommand::RemoveSender(neighbor)).unwrap();
                }
            }

        }
    }

}

#[derive(PartialEq)]
enum Screen {
    NetworkScreen,
    LogsScreen,
}

pub fn main()-> Result<(), eframe::Error> {
    let (node_event_send, node_event_recv) = crossbeam_channel::unbounded();
    let (node_command_send, node_command_recv) = crossbeam_channel::unbounded();

    let simulation_controller = SimulationController::new(HashMap::new(), node_event_recv.clone(), node_command_recv.clone());

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rustaceans Wit Attitudes",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(node_event_recv, node_command_recv))))
    )
}

struct LogEntry {
    timestamp: String,
    message: String,
}

struct MyApp {
    current_screen: Screen,
    logs: Vec<LogEntry>,
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
    node_event_recv: Receiver<DroneEvent>,
    node_command_recv: Receiver<DroneCommand>,
    clients: Vec<String>, // List of clients
    servers: Vec<String>, // List of servers
    open_popups: HashMap<String, bool>,
}

impl MyApp {
    fn new(node_event_recv: Receiver<DroneEvent>, node_command_recv: Receiver<DroneCommand>) -> Self {
        Self {
            current_screen: Screen::NetworkScreen,
            logs: Vec::new(),
            show_confirmation_dialog: false,
            allowed_to_close: false,
            node_event_recv,
            node_command_recv,
            clients: vec!["Client1".to_string(), "Client2".to_string()], // Example clients
            servers: vec!["Server1".to_string()], // Example servers
            open_popups: HashMap::new(), // Initialize the map to track popups
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
                    ui.label(format!("Controls for {}", name));
                    if ui.button("Do Something").clicked() {
                        println!("Action for {}", name);
                    }
                });
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for new events and log them
        while let Ok(event) = self.node_event_recv.try_recv(){
            self.log_event(event); // Use the new log_event method
        }

        while let Ok(command) = self.node_command_recv.try_recv() {
            self.log_command(command);
        }

        egui::TopBottomPanel::top("navigation_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Navigation buttons
                if ui.button("Network Topology").clicked() {
                    self.current_screen = Screen::NetworkScreen;
                }

                if ui.button("Events Log").clicked() {
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
                    });

                },
                Screen::LogsScreen => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for log in &self.logs {
                            // Format the log entry as [timestamp] event_type: message
                            let formatted_log = format!("[{}] {}", log.timestamp, log.message);
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
    }
}