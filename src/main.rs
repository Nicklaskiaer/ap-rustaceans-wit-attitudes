mod network_initializer;
mod types;
use crate::network_initializer::network_initializer::*;

fn main() {
    let c = parse_toml();
    let drone_threads = initialize_drones(c.drone);
    // let servers_threads = initialize_servers(&c.drone);
    // let clients_threads = initialize_clients(&c.drone);
    // fn topology_setup(&drone_threads, &servers_threads, &clients_threads)
    // fn crash_handle()
}
