use std::collections::HashMap;
use wg_2024::controller::SimulationController;
use wg_2024::drone::DroneOptions;
use wg_2024::network::topology::Node;

struct SimController{
    
}

// impl SimController {
//     fn new() -> Self {
//         Self {
//             id: options.id,
//             sim_contr_send: options.sim_contr_send,
//             sim_contr_recv: options.sim_contr_recv,
//             packet_recv: options.packet_recv,
//             pdr: (options.pdr * 100.0) as u8,
//             packet_send: HashMap::new(),
//         }
// }
// 
// impl SimulationController for SimController {
//     fn crash(&mut self, crashed: &str) {
//         todo!()
//     }
// 
//     fn spawn_node(&mut self, new_node: Node) {
//         todo!()
//     }
// 
//     fn message_sent(source: &str, target: &str) {
//         todo!()
//     }
// }