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
mod test_fragments;
mod assembler;
mod testing;
mod client_server;

fn main() {
    debug!("Running in Debug mode");
    network_initializer::network_initializer::main();
    // simulation_controller::simulation_controller::main().expect("GUI panicked!");
}
