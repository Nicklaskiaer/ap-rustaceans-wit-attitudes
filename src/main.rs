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
mod server;
mod message;
mod types;
mod test_fragments;
mod assembler;
mod client;
mod testing;

fn main() {
    debug!("Running in Debug mode");
    network_initializer::network_initializer::main();
    // simulation_controller::simulation_controller::main().expect("GUI panicked!");
}
