use crate::client_server::network_core::{ContentType, ServerType};
use crate::simulation_controller::gui::MyApp;
use crate::simulation_controller::gui_structs::*;
use chrono::{DateTime, Utc};
use chrono_tz::Europe::Rome;
use eframe::egui;
use std::path::Path;
use wg_2024::network::NodeId;

pub fn show_popup(app: &mut MyApp, ctx: &egui::Context, name: &str) {

    // Early return if this popup shouldn't be open
    if !*app.open_popups.get(name).unwrap_or(&false) {
        return;
    }

    // Early return if this is a drone popup but the drone no longer exists
    if name.starts_with("Drone") {
        if let Some(node_id_str) = name.split_whitespace().nth(1) {
            if let Ok(node_id) = node_id_str.parse::<NodeId>() {
                if !app.simulation_controller.get_drones().contains_key(&node_id) {
                    app.open_popups.remove(name);
                    return;
                }
            }
        }
    }

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
                    show_client_controls(app, ui, node_id, ctx);
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
                        if app.simulation_controller.handle_add_sender(node_id, sender_id) {
                            app.topology_needs_update = true;
                        }
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
                        if app.simulation_controller.handle_remove_sender(node_id, sender_id) {
                            app.topology_needs_update = true;
                        }
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
            if let Some((_, _, _)) = app.simulation_controller.get_drones().get(&node_id) {
                let message = format!("[COMMAND] Crashing {}", name);
                app.logs_vec.push(LogEntry {
                    timestamp: formatted_time.to_string(),
                    message,
                });
                app.simulation_controller.handle_crash(node_id);

                // Close the popup by removing its entry
                app.open_popups.remove(name);

                // Also clean up any related state
                app.slider_temp_pdrs.remove(&node_id);
                app.drone_text_inputs.remove(&node_id);
            }
        }
    }
}

fn show_client_controls(
    app: &mut MyApp,
    ui: &mut egui::Ui,
    node_id: NodeId,
    ctx: &egui::Context,  // Add this parameter
) {
    // Track selected screen for this client
    let screen = app.client_popup_screens.entry(node_id).or_insert(ClientPopupScreen::Chatroom);

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
            .selectable_label(*screen == ClientPopupScreen::Content, "Content")
            .clicked()
        {
            *screen = ClientPopupScreen::Content;
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
                    if ui.button("Client List").clicked(){
                        // Set the client list popup to open for this client
                        app.client_list_popups.insert(node_id, true);
                        app.simulation_controller.handle_client_list_request(node_id, server_id_sel.clone());
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

                    let mut display_message = String::from("You are not registered to the server!");

                    if let Some(servers) = app.registered_servers.get(&node_id) {
                        if servers.contains(&server_id_sel) {
                            display_message = String::from("");
                            if let Some(message_list) =
                                app.chatrooms_messages.get_mut(&server_id_sel)
                            {
                                for chat_message in message_list {
                                    if chat_message.content.starts_with("Client"){
                                        ui.label(format!("{}", chat_message.content));
                                    }else {
                                        ui.label(format!("Client {}: {}", chat_message.sender_id, chat_message.content));
                                    }
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
                    app.simulation_controller.handle_send_chat_message(node_id, server_id_sel, text_input.parse().unwrap());
                    text_input.clear();
                }
            });
        }
        ClientPopupScreen::Content => {
            let selected_server_images = app.selected_server.entry(node_id).or_default();
            let mut selected_type_option: Option<ContentType> = None;

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

                        // Filter to only ContentServers
                        for (server_id, (_, _, server_type)) in servers {
                            if let ServerType::ContentServer(content_type) = server_type {
                                let content_type_str = match content_type {
                                    ContentType::Media => "Media",
                                    ContentType::Text => "Text",
                                };
                                let server_id_str = format!("Server {} ({})", server_id, content_type_str);
                                if ui.selectable_label(
                                    selected_server_images == &server_id_str,
                                    &server_id_str,
                                ).clicked() {
                                    *selected_server_images = server_id_str;
                                }
                            }
                        }
                    });
            });


            // Get the selected server ID and content type from the selected string
            let mut selected_server_id = 0;
            if !selected_server_images.is_empty() {
                // Extract server ID
                if let Some(num_str) = selected_server_images.split_whitespace().nth(1) {
                    if let Ok(server_id) = num_str.parse::<u8>() {
                        selected_server_id = server_id;
                    }
                }

                // Extract content type from the selection string
                if selected_server_images.contains("(Media)") {
                    selected_type_option = Some(ContentType::Media);
                } else if selected_server_images.contains("(Text)") {
                    selected_type_option = Some(ContentType::Text);
                }
            }
            ui.separator();
            
            if let Some(selected_type) = selected_type_option {
                match &selected_type {
                    ContentType::Media => {
                        // Request image list button
                        if ui.button("Request Image List").clicked() && selected_server_id > 0 {
                            app.simulation_controller.handle_image_list_request(node_id, selected_server_id);
                        }

                        // Display available images from the selected server
                        if selected_server_id > 0 {
                            match app.clients_downloaded_data.get_know_media_list(node_id, selected_server_id) {
                                None => {
                                    ui.label("No image list available. Click 'Request Image List' to get available images.");
                                }
                                Some(index_list) => {
                                    ui.label("Available Images:");
                                    ui.label(format!("{:?}", index_list));
                                }
                            }
                        }
                        ui.separator();

                        // Image request section
                        ui.label("Request Specific Image:");
                        ui.horizontal(|ui| {
                            let data_id_input = app.client_data_id_inputs.entry(node_id).or_default();
                            ui.label("Image ID:");
                            ui.add(egui::DragValue::new(data_id_input).speed(1.0));

                            if ui.button("Request Image").clicked() && selected_server_id > 0 {
                                app.simulation_controller.handle_image_request(
                                    node_id,
                                    selected_server_id,
                                    *data_id_input,
                                );
                            }
                        });
                        ui.separator();

                        // Display requested images grid
                        ui.label("Requested Images:");
                        // app.clients_downloaded_data.get_all_know_data(node_id)
                        if let Some(image_ids) = app.clients_downloaded_data.get_know_media(node_id, selected_server_id) {
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
                                                let image_buffer = image.to_rgba8();
                                                let (width, height) = image_buffer.dimensions();
                                                let size = [width as usize, height as usize];
                                                let texture = ui.ctx().load_texture(
                                                    format!("image_{}", image_id),
                                                    egui::ColorImage::from_rgba_unmultiplied(
                                                        size,
                                                        &image_buffer.into_raw(),
                                                    ),
                                                    egui::TextureOptions::default(),
                                                );
                                                ui.add(
                                                    egui::Image::new(&texture)
                                                        .fit_to_exact_size(egui::vec2(100.0, 100.0)),
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
                    },
                    ContentType::Text => {
                        // Request Text list button
                        if ui.button("Request Text List").clicked() && selected_server_id > 0 {
                            app.simulation_controller.handle_text_list_request(node_id, selected_server_id);
                        }

                        // Display available text from the selected server
                        if selected_server_id > 0 {
                            match app.clients_downloaded_data.get_know_text_list(node_id, selected_server_id) {
                                None => {
                                    ui.label("No Text list available. Click 'Request Text List' to get available images.");
                                }
                                Some(index_list) => {
                                    ui.label("Available Texts:");
                                    ui.label(format!("{:?}", index_list));
                                }
                            }
                        }
                        ui.separator();

                        // Text request section
                        ui.label("Request Specific Text:");
                        ui.horizontal(|ui| {
                            let data_id_input = app.client_data_id_inputs.entry(node_id).or_default();
                            ui.label("Text ID:");
                            ui.add(egui::DragValue::new(data_id_input).speed(1.0));

                            if ui.button("Request Text").clicked() && selected_server_id > 0 {
                                app.simulation_controller.handle_text_request(
                                    node_id,
                                    selected_server_id,
                                    *data_id_input,
                                );
                            }
                        });
                        ui.separator();

                        // Display requested files
                        ui.label("Requested Texts:");
                        // app.clients_downloaded_data.get_all_know_data(node_id)
                        if let Some(files) = app.clients_downloaded_data.get_know_text(node_id, selected_server_id) {
                            if files.is_empty() {
                                ui.label("No Texts have been requested yet.");
                            } else {
                                egui::ScrollArea::vertical()
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        for file_id in files {
                                            let file_path = format!("server_content/text_files/{}", file_id);
                                            ui.collapsing(format!("File {}", file_id), |ui| {
                                                // Read the content from the file
                                                match std::fs::read_to_string(&file_path) {
                                                    Ok(content) => {
                                                        ui.add(
                                                            egui::TextEdit::multiline(&mut content.clone())
                                                                .desired_width(ui.available_width())
                                                                .desired_rows(10)
                                                        );

                                                        // Extract image IDs using regex
                                                        let re = regex::Regex::new(r"\[image_(\d+)]").unwrap();
                                                        let mut image_ids = Vec::new();

                                                        for cap in re.captures_iter(&content) {
                                                            if let Some(id_str) = cap.get(1) {
                                                                if let Ok(id) = id_str.as_str().parse::<u64>() {
                                                                    image_ids.push(id);
                                                                }
                                                            }
                                                        }

                                                        // Display images if any were found
                                                        if !image_ids.is_empty() {
                                                            ui.separator();
                                                            ui.label("Referenced Images:");

                                                            let columns = 3;
                                                            egui::Grid::new(format!("text_images_grid_{}", file_id))
                                                                .num_columns(columns)
                                                                .spacing([10.0, 10.0])
                                                                .show(ui, |ui| {
                                                                    for (i, image_id) in image_ids.iter().enumerate() {
                                                                        // Check all servers for the image
                                                                        let mut found_image = false;

                                                                        // Get all content servers
                                                                        let servers = app.simulation_controller.get_servers();
                                                                        for (server_id, (_, _, server_type)) in servers {
                                                                            // Only check media servers
                                                                            if let ServerType::ContentServer(ContentType::Media) = server_type {
                                                                                // Try to get the image from this media server
                                                                                if let Some(media_id) = app.clients_downloaded_data
                                                                                    .get_know_media_with_id(node_id, *server_id, *image_id)
                                                                                {
                                                                                    let image_path = format!(
                                                                                        "server_content/media_files/{}.jpg",
                                                                                        media_id
                                                                                    );

                                                                                    if let Ok(image) = image::open(&Path::new(&image_path)) {
                                                                                        let image_buffer = image.to_rgba8();
                                                                                        let (width, height) = image_buffer.dimensions();
                                                                                        let size = [width as usize, height as usize];
                                                                                        let texture = ui.ctx().load_texture(
                                                                                            format!("text_image_{}_{}", file_id, image_id),
                                                                                            egui::ColorImage::from_rgba_unmultiplied(
                                                                                                size,
                                                                                                &image_buffer.into_raw(),
                                                                                            ),
                                                                                            egui::TextureOptions::default(),
                                                                                        );
                                                                                        ui.add(
                                                                                            egui::Image::new(&texture)
                                                                                                .fit_to_exact_size(egui::vec2(100.0, 100.0)),
                                                                                        );
                                                                                        found_image = true;
                                                                                        break;  // Exit the loop once we've found and displayed the image
                                                                                    }
                                                                                }
                                                                            }
                                                                        }

                                                                        if !found_image {
                                                                            ui.label(format!("Image {} not available", image_id));
                                                                        }

                                                                        if (i + 1) % columns == 0 {
                                                                            ui.end_row();
                                                                        }
                                                                    }
                                                                });
                                                        }
                                                    },
                                                    Err(_) => {
                                                        ui.label(format!("Error reading file content for ID {}", file_id));
                                                    }
                                                }
                                            });
                                        }
                                    });
                            }
                            //     egui::ScrollArea::vertical()
                            //         .max_height(200.0)
                            //         .show(ui, |ui| {
                            //             for file_id in files {
                            //                 let file_path = format!("server_content/text_files/{}", file_id);
                            //                 ui.collapsing(format!("File {}", file_id), |ui| {
                            //                     // Read the content from the file
                            //                     match std::fs::read_to_string(&file_path) {
                            //                         Ok(content) => {
                            //                             ui.add(
                            //                                 egui::TextEdit::multiline(&mut content.clone())
                            //                                     .desired_width(ui.available_width())
                            //                                     .desired_rows(10)
                            //                             );
                            //                         },
                            //                         Err(_) => {
                            //                             ui.label(format!("Error reading file content for ID {}", file_id));
                            //                         }
                            //                     }
                            //                 });
                            //             }
                            //         });
                            // }
                        } else {
                            ui.label("No Texts available for this client.");
                        }
                    }
                }
            }
        }
    }

    // Show client list popup if it's open for this client
    if let Some(true) = app.client_list_popups.get(&node_id) {
        egui::Window::new(format!("Client List for Client {}", node_id))
            .resizable(true)
            .collapsible(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.label("Registered Clients:");
                ui.separator();

                if app.registered_clients.is_empty() {
                    ui.label("No clients registered");
                } else {
                    for client_id in &app.registered_clients {
                        ui.label(format!("Client {}", client_id));
                    }
                }

                ui.separator();
                if ui.button("Close").clicked() {
                    app.client_list_popups.insert(node_id, false);
                }
            });
    }
}
