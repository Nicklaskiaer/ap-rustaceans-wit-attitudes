mod network_initializer;
mod simulation_controller;
mod server;
mod message;
mod types;
mod test_fragments;

fn main() {
    println!("Hello, world!");
    
    network_initializer::network_initializer::main();
}
