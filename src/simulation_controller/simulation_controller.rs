use std::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

use crate::simulation_controller::gui::MyApp;

pub struct SimulationController {
    drones: HashMap<NodeId, Sender<DroneCommand>>,
    node_event_recv: Receiver<DroneEvent>
}

impl SimulationController {
    pub fn new(drones: HashMap<NodeId, Sender<DroneCommand>>, node_event_recv: Receiver<DroneEvent>)->Self{
        SimulationController{
            drones,
            node_event_recv
        }
    }

    pub fn handle_remove_sender(&self, drone_sender_id: NodeId, drone_id: NodeId) {
        if let Some(drone_sender) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::RemoveSender(drone_id)).unwrap();
        }
    }

    pub fn handle_add_sender(&self, drone_sender_id: NodeId, drone_id: NodeId, drone_packet: Sender<Packet>) {
        if let Some(drone_sender) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::AddSender(drone_id, drone_packet)).unwrap();
        }
    }

    pub fn handle_set_packet_drop_rate(&self, drone_sender_id: NodeId, drop_rate: f32) {
        if let Some(drone_sender) = self.drones.get(&drone_sender_id) {
            drone_sender.send(DroneCommand::SetPacketDropRate(drop_rate)).unwrap();
        }
    }

    pub fn handle_crash(&mut self, drone_sender_id: NodeId, neighbors: Vec<NodeId>) {
        if let Some(crashed_drone_sender) = self.drones.get(&drone_sender_id) {
            // send crash command to the drone
            crashed_drone_sender.send(DroneCommand::Crash).unwrap();

            for neighbor in neighbors {
                if let Some(neighbor_drone_sender) = self.drones.get(&neighbor) {
                    // remove drone from neighbor
                    neighbor_drone_sender.send(DroneCommand::RemoveSender(drone_sender_id)).unwrap();

                    // remove neighbor form drone
                    crashed_drone_sender.send(DroneCommand::RemoveSender(neighbor)).unwrap();
                }
            }

        }
    }

    pub fn get_drone_ids(&self) -> Vec<String> {
        self.drones.keys()
            .map(|node_id| format!("{}", node_id.to_string()))
            .collect()
    }

    pub fn get_node_event_recv(&self) -> Receiver<DroneEvent>{
        self.node_event_recv.clone()
    }
}


pub fn simulation_controller_main(controller_drones: HashMap<NodeId, Sender<DroneCommand>>, node_event_recv: Receiver<DroneEvent>) -> Result<(), eframe::Error> {

    let simulation_controller = SimulationController::new(
            controller_drones,
            node_event_recv.clone()
    );

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rustaceans Wit Attitudes",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(simulation_controller))))
    )
}