use eframe::egui;
mod network_initializer;
mod types;

#[derive(PartialEq)]
enum Screen {
    FirstScreen,
    SecondScreen,
}

fn main()-> Result<(), eframe::Error> {
    //network_initializer::network_initializer::main();

    let options = eframe::NativeOptions {
        //initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Rustaceans Wit Attitudes",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default()))
    )
}

struct MyApp {
    current_screen: Screen,
    logs: Vec<String>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            current_screen: Screen::FirstScreen,
            logs: Vec::new(), // Initialize an empty log vector
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("navigation_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Navigation buttons
                if ui.button("Network Topology").clicked() {
                    self.current_screen = Screen::FirstScreen;
                }

                if ui.button("Events Log").clicked() {
                    self.current_screen = Screen::SecondScreen;
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_screen {
                Screen::FirstScreen => {
                    ui.heading("Network Topology");
                    ui.label("This is the screen for the network topology");

                    if ui.button("new event").clicked() {
                        self.logs.push("Drone X did event Y.".to_string());
                    }
                },

                Screen::SecondScreen => {
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(34, 34, 34)) // Set your desired color here
                        .inner_margin(10.0) // Optional padding for the heading area
                        .show(ui, |ui| {
                            ui.heading("Events Log");
                        });;

                    // Scroll area for logs
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        let mut n: i32 = 1;

                        for log in &self.logs {
                            ui.label(format!("[{}] - {} ", n.to_string(), log));
                            n+=1;
                        }
                    });
                }
            }
        });
    }
}