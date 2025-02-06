mod network_initializer;
mod simulation_controller;
mod server;
mod message;
mod types;
mod test_fragments;
mod assembler;
mod client;

fn main() {

    network_initializer::network_initializer::main();
    // simulation_controller::simulation_controller::main().expect("GUI panicked!");
}
