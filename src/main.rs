mod assembler;
mod message;
mod network_initializer;
mod server;
mod simulation_controller;
mod test_fragments;
mod types;

fn main() {
    println!("Hello, world!");

    network_initializer::network_initializer::main();
}
