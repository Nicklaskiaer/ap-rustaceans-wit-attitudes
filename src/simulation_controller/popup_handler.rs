use wg_2024::network::{NodeId, SourceRoutingHeader};

use crate::simulation_controller::gui_structs::*;
use crate::simulation_controller::gui::MyApp;

use eframe::egui;
use chrono::{DateTime, Local, Utc};
use chrono_tz::Europe::Rome;

pub fn show_popup(app: &mut MyApp, ctx: &egui::Context, name: &str) {
    let current_time: DateTime<Utc> = Utc::now();
    let italian_time = current_time.with_timezone(&Rome);
    let formatted_time = italian_time.format("%d-%m-%y %H:%M:%S").to_string();

    if let Some(is_open) = app.open_popups.get_mut(name) {
        egui::Window::new(format!("Controls for {}", name)).resizable(true).collapsible(true).open(is_open)
            .show(ctx, |ui| {
                //Extract the node ID from the name.
                if let Some(node_id_str) = name.split_whitespace().nth(1) {
                    if let Ok(node_id) = node_id_str.parse::<NodeId>() {

                        //Handle Drones controls.
                        if name.starts_with("Drone") {
                            let drop_rate = app
                                .simulation_controller
                                .get_drones()
                                .get(&node_id)
                                .map(|(_, _, rate)| *rate);

                            let input_text = app
                                .drone_text_inputs
                                .entry(node_id)
                                .or_insert_with(String::new);

                            if let Some(drop_rate) = drop_rate{
                                ui.label(format!("Current PDR: {:.2}%", drop_rate * 100.0));

                                //Handle Set Packet Drop Rate.
                                let entry = app.slider_temp_pdrs.entry(node_id).or_insert(drop_rate);

                                ui.horizontal(|ui| {
                                    ui.label("New Drop Rate:");
                                    ui.add(egui::Slider::new(entry, 0.0..=1.0).text(""));

                                    if ui.button("Update").clicked() {
                                        if (*entry - drop_rate).abs() > f32::EPSILON {
                                            app.simulation_controller.handle_set_packet_drop_rate(node_id, *entry);

                                            app.logs_vec.push(LogEntry {
                                                timestamp: formatted_time.clone(),
                                                message: format!("[COMMAND] Updated PDR of Drone {} to {:.2}%", node_id, *entry * 100.0),
                                            });
                                        }
                                    }
                                });

                                //Handle Add/Remove a Sender
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(input_text);
                                });

                                ui.horizontal(|ui| {
                                    if ui.button("Add Sender").clicked() {
                                        match input_text.parse::<NodeId>() {
                                            Ok(node_id) => {
                                                // self.simulation_controller.handle_add_sender(...) //todo!

                                                let message = format!("[COMMAND] Added sender {} to {}", node_id, name);
                                                app.logs_vec.push(LogEntry {
                                                    timestamp: formatted_time.clone(),
                                                    message,
                                                });

                                                *input_text = String::new(); //Clear input after action.
                                            }
                                            Err(_) => println!("Invalid input! Please enter a valid NodeId."),
                                        }
                                    }

                                    if ui.button("Remove Sender").clicked() {
                                        match input_text.parse::<NodeId>() {
                                            Ok(node_id) => {
                                                // self.simulation_controller.handle_remove_sender(...) //todo!

                                                let message = format!("[COMMAND] Removed sender {} from {}", node_id, name);
                                                app.logs_vec.push(LogEntry {
                                                    timestamp: formatted_time.clone(),
                                                    message,
                                                });

                                                *input_text = String::new(); //Clear input after action.
                                            }
                                            Err(_) => println!("Invalid input! Please enter a valid NodeId."),
                                        }
                                    }
                                });

                                //Handle Crash button.
                                if ui.button("Crash").clicked() {
                                    if let Some((_, neighbors, _)) = app.simulation_controller.get_drones().get(&node_id) {
                                        let message = format!("[COMMAND] Crashing {}", name);
                                        app.logs_vec.push(LogEntry {
                                            timestamp: formatted_time.clone(),
                                            message,
                                        });
                                        app.simulation_controller.handle_crash(node_id, neighbors.clone());
                                    }
                                }
                            }
                        }

                        //Handle Clients controls.
                        else if name.starts_with("Client") {
                            let popup_flag = app
                                .open_serverlist_popups
                                .entry(node_id)
                                .or_insert(false);

                            if ui.button("Open Server List").clicked() {
                                *popup_flag = true;
                            }

                            let mut open_popup = *popup_flag;

                            //List of server that the client can talk to.
                            egui::Window::new(format!("Server List of Client {}", node_id))
                                .open(&mut open_popup)
                                .collapsible(false)
                                .resizable(false)
                                .show(ctx, |ui| {
                                    ui.separator();
                                    ui.label("Available Servers:");

                                    for server_id in app.simulation_controller.get_server_ids() {
                                        if let Ok(server_node_id) = server_id.split_whitespace().nth(1).unwrap_or("0").parse::<NodeId>() {
                                            if ui.button(&server_id).clicked() {
                                                // Store that this client-server pair's popup should be open
                                                app.server_action_popups.insert((node_id, server_node_id), true);
                                            }
                                        }
                                    }
                                });

                            //Sync back visibility state
                            *popup_flag = open_popup;

                            //Show server action popups for this client.
                            let mut popups_to_remove = Vec::new();
                            for ((c_id, s_id), is_open) in app.server_action_popups.iter_mut() {
                                if *c_id == node_id {
                                    let mut open = *is_open;
                                    egui::Window::new(format!("Client {} - Server {} Controls", c_id, s_id))
                                        .open(&mut open)
                                        .collapsible(false)
                                        .resizable(false)
                                        .show(ctx, |ui| {
                                            //todo!(add controls client-server)
                                        });

                                    if !open {
                                        popups_to_remove.push((*c_id, *s_id));
                                    }
                                    *is_open = open;
                                }
                            }

                            // Remove closed popups
                            for key in popups_to_remove {
                                app.server_action_popups.remove(&key);
                            }
                        }

                        //Handle Server controls.
                        else if name.starts_with("Server") {
                            ui.label("Server controls coming soon...");
                        }
                    }
                }
            });
    }
}