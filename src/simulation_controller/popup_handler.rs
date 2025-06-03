use crate::simulation_controller::gui::MyApp;
use crate::simulation_controller::gui_structs::*;
use chrono::{DateTime, Utc};
use chrono_tz::Europe::Rome;
use eframe::egui;
use wg_2024::network::NodeId;

pub fn show_popup(app: &mut MyApp, ctx: &egui::Context, name: &str) {
    let current_time: DateTime<Utc> = Utc::now();
    let italian_time = current_time.with_timezone(&Rome);
    let formatted_time = italian_time.format("%d-%m-%y %H:%M:%S").to_string();

    // Store the is_open value in a local variable
    let mut is_open = *app.open_popups.get(name).unwrap_or(&false);

    // Create the window
    let window = egui::Window::new(format!("Controls for {}", name))
        .resizable(true)
        .collapsible(true)
        .default_width(400.0)
        .min_width(300.0)
        .open(&mut is_open);

    // Show the window and handle the content
    window.show(ctx, |ui| {
        // Extract the node ID from the name
        if let Some(node_id_str) = name.split_whitespace().nth(1) {
            if let Ok(node_id) = node_id_str.parse::<NodeId>() {
                // Handle Drones controls
                if name.starts_with("Drone") {
                    show_drone_controls(app, ui, node_id, name, &formatted_time);
                }
                // Handle Clients controls
                else if name.starts_with("Client") {
                    show_client_controls(app, ui, node_id);
                }
                // Handle Server controls
                else if name.starts_with("Server") {
                    ui.label("Server controls coming soon...");
                }
            }
        }
    });

    // Update the original is_open value
    if let Some(open) = app.open_popups.get_mut(name) {
        *open = is_open;
    }
}

fn show_drone_controls(
    app: &mut MyApp,
    ui: &mut egui::Ui,
    node_id: NodeId,
    name: &str,
    formatted_time: &str,
) {
    let drop_rate = app
        .simulation_controller
        .get_drones()
        .get(&node_id)
        .map(|(_, _, rate)| *rate);

    let input_text = app
        .drone_text_inputs
        .entry(node_id)
        .or_insert_with(String::new);

    if let Some(drop_rate) = drop_rate {
        ui.label(format!("Current PDR: {:.2}%", drop_rate * 100.0));

        // Handle Set Packet Drop Rate
        let entry = app.slider_temp_pdrs.entry(node_id).or_insert(drop_rate);

        ui.horizontal(|ui| {
            ui.label("New Drop Rate:");
            ui.add(egui::Slider::new(entry, 0.0..=1.0).text(""));

            if ui.button("Update").clicked() {
                if (*entry - drop_rate).abs() > f32::EPSILON {
                    app.simulation_controller.handle_set_packet_drop_rate(node_id, *entry);
                    app.logs_vec.push(LogEntry {
                        timestamp: formatted_time.to_string(),
                        message: format!("[COMMAND] Updated PDR of Drone {} to {:.2}%", node_id, *entry * 100.0),
                    });
                }
            }
        });

        // Handle Add/Remove a Sender
        ui.horizontal(|ui| {
            ui.text_edit_singleline(input_text);
        });

        ui.horizontal(|ui| {
            if ui.button("Add Sender").clicked() {
                match input_text.parse::<NodeId>() {
                    Ok(sender_id) => {
                        let message = format!("[COMMAND] Added sender {} to {}", sender_id, name);
                        app.logs_vec.push(LogEntry {
                            timestamp: formatted_time.to_string(),
                            message,
                        });
                        *input_text = String::new();
                    }
                    Err(_) => println!("Invalid input!"),
                }
            }

            if ui.button("Remove Sender").clicked() {
                match input_text.parse::<NodeId>() {
                    Ok(sender_id) => {
                        let message = format!("[COMMAND] Removed sender {} from {}", sender_id, name);
                        app.logs_vec.push(LogEntry {
                            timestamp: formatted_time.to_string(),
                            message,
                        });
                        *input_text = String::new();
                    }
                    Err(_) => println!("Invalid input!"),
                }
            }
        });

        // Handle Crash button
        if ui.button("Crash").clicked() {
            if let Some((_, neighbors, _)) = app.simulation_controller.get_drones().get(&node_id) {
                let message = format!("[COMMAND] Crashing {}", name);
                app.logs_vec.push(LogEntry {
                    timestamp: formatted_time.to_string(),
                    message,
                });
                app.simulation_controller.handle_crash(node_id, neighbors.clone());
            }
        }
    }
}

fn show_client_controls(
    app: &mut MyApp,
    ui: &mut egui::Ui,
    node_id: NodeId,
) {
    // Track selected screen for this client
    let screen = app.client_popup_screens.entry(node_id).or_insert(ClientPopupScreen::Chatroom);

    // Navigation bar
    ui.horizontal(|ui| {
        if ui.selectable_label(*screen == ClientPopupScreen::Chatroom, "Main").clicked() {
            *screen = ClientPopupScreen::Chatroom;
        }
        if ui.selectable_label(*screen == ClientPopupScreen::Other, "Advanced").clicked() {
            *screen = ClientPopupScreen::Other;
        }
    });

    ui.separator();

    match screen {
        ClientPopupScreen::Chatroom => {
            let selected_server = app.selected_server.entry(node_id).or_default();

            ui.horizontal(|ui| {
                // Dropdown menu for servers
                egui::ComboBox::from_label("")
                    .width(100.0)
                    .selected_text(if selected_server.is_empty() {
                        "Select Server".to_string()
                    } else {
                        selected_server.clone()
                    })
                    .show_ui(ui, |ui| {
                        for server_id in app.simulation_controller.get_server_ids() {
                            if ui.selectable_label(selected_server == &server_id, &server_id).clicked() {
                                *selected_server = server_id.clone();
                            }
                        }
                    });
                if ui.button("Register").clicked() {
                    //todo!(Manage connection)
                }
            });

            ui.separator();

            // Message history
            egui::ScrollArea::vertical()
                .stick_to_right(true)
                .max_height(100.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    //todo!(Insert messages)
                    ui.label("Client 1: Test");
                    ui.label("Client 2: Test");
                    ui.label("Client 1: Test");
                    ui.label("Client 2: Test");
                    ui.label("Client 1: Test");
                    ui.label("Client 2: Test");
                    ui.label("Client 1: Test");
                    ui.label("Client 2: Test");
                    ui.label("Client 1: Test");
                    ui.label("Client 2: Test");
                    ui.label("Client 1: Test");
                    ui.label("Client 2: Test");
                    ui.label("Client 1: Test");
                });

            ui.separator();

            // Message input
            ui.horizontal(|ui| {
                let text_input = app.client_message_inputs.entry(node_id).or_default();
                ui.add(
                    egui::TextEdit::singleline(text_input)
                        .desired_width(ui.available_width() - 70.0)
                        .hint_text("Message")
                );
                ui.add_space(2.0);
                if ui.button("Send").clicked() {
                    //todo!(Send message)
                    *text_input = String::new();
                }
            });
        }
        ClientPopupScreen::Other => {
            ui.label("Advanced controls coming soon...");
        }
    }
}