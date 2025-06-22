use std::env;

#[cfg(feature = "debug")]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { println!("[DEBUG] {}", format!($($arg)*)) }
}

#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {}
}

mod network_initializer;
mod simulation_controller;
mod message;
mod assembler;
mod client_server;

fn main() {
    debug!("Running in Debug mode");

    // Get config file path from command line arguments or use default
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "src/config.toml".to_string());

    debug!("Using configuration file: {}", config_path);
    network_initializer::network_initializer::main(config_path);
}
