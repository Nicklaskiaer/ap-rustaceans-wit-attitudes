use crate::client_server::network_core::{ChatMessage, ContentType, ServerType};
use crate::simulation_controller::gui::MyApp;
use crate::simulation_controller::gui_structs::*;
use chrono::{DateTime, Utc};
use chrono_tz::Europe::Rome;
use eframe::egui;
use std::path::Path;
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
                    app.simulation_controller
                        .handle_set_packet_drop_rate(node_id, *entry);
                    app.logs_vec.push(LogEntry {
                        timestamp: formatted_time.to_string(),
                        message: format!(
                            "[COMMAND] Updated PDR of Drone {} to {:.2}%",
                            node_id,
                            *entry * 100.0
                        ),
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
                        let message =
                            format!("[COMMAND] Removed sender {} from {}", sender_id, name);
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
                app.simulation_controller
                    .handle_crash(node_id, neighbors.clone());
            }
        }
    }
}

fn show_client_controls(app: &mut MyApp, ui: &mut egui::Ui, node_id: NodeId) {
    // Track selected screen for this client
    let screen = app
        .client_popup_screens
        .entry(node_id)
        .or_insert(ClientPopupScreen::Chatroom);

    // Track selected server for this client
    let mut server_id_sel: u8 = 0;

    // Navigation bar
    ui.horizontal(|ui| {
        if ui
            .selectable_label(*screen == ClientPopupScreen::Chatroom, "Chatroom")
            .clicked()
        {
            *screen = ClientPopupScreen::Chatroom;
        }
        if ui
            .selectable_label(*screen == ClientPopupScreen::Other, "Images")
            .clicked()
        {
            *screen = ClientPopupScreen::Other;
        }
    });

    ui.separator();

    match screen {
        ClientPopupScreen::Chatroom => {
            let selected_server = app.selected_server.entry(node_id).or_default();
            let registered_servers = app.registered_servers.entry(node_id).or_default();

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
                        // Get all server IDs and their types
                        let servers = app.simulation_controller.get_servers();

                        // Filter to only CommunicationServers
                        for (server_id, (_, _, server_type)) in servers {
                            if let ServerType::CommunicationServer = server_type {
                                let server_id_str = format!("Server {}", server_id);
                                if ui
                                    .selectable_label(
                                        selected_server == &server_id_str,
                                        &server_id_str,
                                    )
                                    .clicked()
                                {
                                    *selected_server = server_id_str;
                                }
                            }
                        }
                    });

                // Converted selected_server to u8
                if let Some(num_str) = selected_server.split_whitespace().last() {
                    if let Ok(server_id) = num_str.parse::<u8>() {
                        server_id_sel = server_id;
                    }
                }

                // If register is pressed, server id is pushed in vec and request is sent to server.
                if ui.button("Register").clicked() {
                    if !registered_servers.contains(&server_id_sel) {
                        app.simulation_controller
                            .handle_registration_request(node_id, server_id_sel.clone());
                    }
                }

                // After client has registered to server then "Client List" button is displayed.
                if registered_servers.contains(&server_id_sel) {
                    if ui.button("Client List").clicked() {
                        //todo!(FIND A WAY TO SHOW THE USER THE CLIENTS THAT ARE REGISTERED TO THE CHATROOM)
                    }
                }
            });

            ui.separator();

            // Message history
            egui::ScrollArea::vertical()
                .stick_to_right(true)
                .max_height(100.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    // debug!("bbbbbbbbbbbb {:?}, {:?}, {:?}", server_id_sel,
                    //     app.registered_servers.clone(),
                    //     app.registered_servers.clone().get(&server_id_sel)
                    // );
                    let mut display_message = String::from("You are not registered to the server!");
                    if let Some(servers) = app.registered_servers.get(&node_id) {
                        if servers.contains(&server_id_sel) {
                            display_message = String::from("");
                            if let Some(message_list) =
                                app.chatrooms_messages.get_mut(&server_id_sel)
                            {
                                for chat_message in message_list {
                                    ui.label(format!(
                                        "{}: {}",
                                        chat_message.sender_id, chat_message.content
                                    ));
                                }
                            }
                        }
                    }
                    ui.label(display_message);
                });

            ui.separator();

            // Message input
            ui.horizontal(|ui| {
                let text_input = app.client_message_inputs.entry(node_id).or_default();
                ui.add(
                    egui::TextEdit::singleline(text_input)
                        .desired_width(ui.available_width() - 70.0)
                        .hint_text("Message"),
                );

                ui.add_space(2.0);

                if ui.button("Send").clicked() {
                    app.simulation_controller.handle_send_chat_message(
                        node_id,
                        server_id_sel,
                        text_input.parse().unwrap(),
                    )
                }
            });
        }
        ClientPopupScreen::Other => {
            let selected_server_images = app.selected_server.entry(node_id).or_default();

            // Server selection dropdown
            ui.horizontal(|ui| {
                ui.label("Select Content Server:");
                egui::ComboBox::from_label("")
                    .width(150.0)
                    .selected_text(if selected_server_images.is_empty() {
                        "Select Server".to_string()
                    } else {
                        selected_server_images.clone()
                    })
                    .show_ui(ui, |ui| {
                        // Get all server IDs and their types
                        let servers = app.simulation_controller.get_servers();

                        // Filter to only ContentServers with Media type
                        for (server_id, (_, _, server_type)) in servers {
                            if let ServerType::ContentServer(content_type) = server_type {
                                if let ContentType::Media = content_type {
                                    let server_id_str = format!("Server {}", server_id);
                                    if ui
                                        .selectable_label(
                                            selected_server_images == &server_id_str,
                                            &server_id_str,
                                        )
                                        .clicked()
                                    {
                                        *selected_server_images = server_id_str;
                                    }
                                }
                            }
                        }
                    });
            });

            // Get the selected server ID
            let mut selected_server_id = 0;
            if let Some(num_str) = selected_server_images.split_whitespace().last() {
                if let Ok(server_id) = num_str.parse::<u8>() {
                    selected_server_id = server_id;
                }
            }

            ui.separator();

            // Request image list button
            if ui.button("Request Image List").clicked() && selected_server_id > 0 {
                app.simulation_controller
                    .handle_image_list_request(node_id, selected_server_id);
            }

            // Display available images from the selected server
            if selected_server_id > 0 {
                if let Some(image_list) = app.client_image_lists.get(&(node_id, selected_server_id))
                {
                    ui.label("Available Images:");
                    ui.label(format!("{:?}", image_list));
                } else {
                    ui.label("No image list available. Click 'Request Image List' to get available images.");
                }
            }

            ui.separator();

            // Image request section
            ui.label("Request Specific Image:");
            ui.horizontal(|ui| {
                let image_id_input = app.client_image_id_inputs.entry(node_id).or_default();
                ui.label("Image ID:");
                ui.add(egui::DragValue::new(image_id_input).speed(1.0));

                if ui.button("Request Image").clicked() && selected_server_id > 0 {
                    app.simulation_controller.handle_image_request(
                        node_id,
                        selected_server_id,
                        *image_id_input,
                    );
                }
            });

            ui.separator();

            // Display requested images grid
            ui.label("Requested Images:");
            if let Some(image_ids) = app.client_images.get(&node_id) {
                if image_ids.is_empty() {
                    ui.label("No images have been requested yet.");
                } else {
                    let columns = 3;

                    egui::Grid::new("client_images_grid")
                        .num_columns(columns)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            for (i, image_id) in image_ids.iter().enumerate() {
                                let image_path =
                                    format!("server_content/media_files/{}.jpg", image_id);

                                if let Ok(image) = image::open(&Path::new(&image_path)) {
                                    let size = [100.0, 100.0];

                                    let texture = ui.ctx().load_texture(
                                        format!("image_{}", image_id),
                                        egui::ColorImage::from_rgba_unmultiplied(
                                            [size[0] as usize, size[1] as usize],
                                            &image.to_rgba8().into_raw(),
                                        ),
                                        egui::TextureOptions::default(),
                                    );

                                    ui.add(
                                        egui::Image::new(&texture)
                                            .fit_to_exact_size(egui::vec2(size[0], size[1])),
                                    );
                                } else {
                                    ui.label(format!("Image {} not found", image_id));
                                }

                                if (i + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                }
            } else {
                ui.label("No images available for this client.");
            }
        }
    }
}
