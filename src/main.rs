mod assembler;
mod message;
mod network_initializer;
mod server;
mod simulation_controller;
mod test_fragments;
mod types;

fn main() {
    network_initializer::network_initializer::main();
}
