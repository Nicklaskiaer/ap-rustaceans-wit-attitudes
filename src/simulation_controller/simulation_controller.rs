use std::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use eframe::{egui};
use egui::color_picker::color_edit_button_rgb;

pub struct SimulationController {
    drones: HashMap<NodeId, Sender<DroneCommand>>,
    node_event_recv: Receiver<DroneEvent>,
}

impl SimulationController {
    pub fn new(drones: HashMap<NodeId, Sender<DroneCommand>>, node_event_recv: Receiver<DroneEvent>)->Self{
        SimulationController{
            drones,
            node_event_recv
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

    // pub fn send_command(command: DroneCommand, sender: &Sender<DroneCommand>){
    //     sender.send(command).unwrap()
    // }
}

#[derive(PartialEq)]
enum Screen {
    NetworkScreen,
    LogsScreen,
}

pub fn main()-> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Rustaceans Wit Attitudes",
        native_options,
        Box::new(|_cc| Ok(Box::<MyApp>::default()))
    )
}

struct MyApp {
    current_screen: Screen,
    logs: Vec<String>,
    show_confirmation_dialog: bool,
    allowed_to_close: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            current_screen: Screen::NetworkScreen,
            logs: Vec::new(), // Initialize an empty log vector
            show_confirmation_dialog: false,
            allowed_to_close: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    ui.heading("Network Topology");
                    ui.label("This is the screen for the network topology");
                    let mut x = 1;

                    if ui.button("Logs Test").clicked() {
                        self.logs.push("Logs Test".to_string());
                        x += 1;
                    }
                },

                Screen::LogsScreen => {
                    // Scroll area for logs, styled differently
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.set_min_size(ui.available_size()); // Make it fill with the rest of the panel
                        for log in &self.logs {
                            ui.label(log);
                        }
                    });
                }
            }
        });
    }
}