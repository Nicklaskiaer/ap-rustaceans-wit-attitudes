use std::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, NodeEvent};
use wg_2024::network::NodeId;

pub struct SimulationController {
    drones: HashMap<NodeId, Sender<DroneCommand>>,
    node_event_recv: Receiver<NodeEvent>,
}

impl SimulationController {
    pub fn new(drones: HashMap<NodeId, Sender<DroneCommand>>, node_event_recv: Receiver<NodeEvent>)->Self{
        SimulationController{
            drones,
            node_event_recv
        }
    }
    
    pub fn crash_all(&mut self) {
        for (_, sender) in self.drones.iter() {
            sender.send(DroneCommand::Crash).unwrap();
        }
    }
}