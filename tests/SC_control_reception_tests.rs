use ap_project_rustaceans_wit_attitudes::drone::MyDrone;
use crossbeam_channel::unbounded;
use std::collections::HashMap;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use wg_2024::controller::DroneCommand;
use wg_2024::drone::Drone;
const TIMEOUT: Duration = Duration::from_millis(400);

#[cfg(test)]
#[test]
pub fn set_pdr_command_test() {
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    //Drone Events
    let (d11_event_send, d11_event_recv) = unbounded();
    //Creates Drone 11
    let mut drone = MyDrone::new(
        11,
        d11_event_send,
        d11_command_recv,
        d11_recv.clone(),
        HashMap::from([]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Test Nack
    let set_pdr_command = DroneCommand::SetPacketDropRate(0.75);
    _d11_command_send.send(set_pdr_command).unwrap();
    sleep(Duration::from_millis(4000));
}

#[cfg(test)]
#[test]
pub fn crash_command_test() {
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    //Drone Events
    let (d11_event_send, d11_event_recv) = unbounded();
    //Creates Drone 11
    let mut drone = MyDrone::new(
        11,
        d11_event_send,
        d11_command_recv,
        d11_recv.clone(),
        HashMap::from([]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Test Nack
    let crash_drone_command = DroneCommand::Crash;
    _d11_command_send.send(crash_drone_command).unwrap();
    sleep(Duration::from_millis(4000));
}

#[cfg(test)]
#[test]
pub fn remove_sender_command_test() {
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //Drone 12
    let (d12_send, d12_recv) = unbounded();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    //Drone Events
    let (d11_event_send, d11_event_recv) = unbounded();
    //Creates Drone 11
    let mut drone = MyDrone::new(
        11,
        d11_event_send,
        d11_command_recv,
        d11_recv.clone(),
        HashMap::from([(12, d12_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Test Nack
    let remove_sender_command = DroneCommand::RemoveSender(12);
    _d11_command_send.send(remove_sender_command).unwrap();
    sleep(Duration::from_millis(4000));
}

#[cfg(test)]
#[test]
pub fn add_channel_command_test() {
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //Drone 12
    let (d12_send, d12_recv) = unbounded();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    //Drone Events
    let (d11_event_send, d11_event_recv) = unbounded();
    //Creates Drone 11
    let mut drone = MyDrone::new(
        11,
        d11_event_send,
        d11_command_recv,
        d11_recv.clone(),
        HashMap::from([]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Test Nack
    let remove_sender_command = DroneCommand::AddSender(12, d12_send.clone());
    _d11_command_send.send(remove_sender_command).unwrap();
    sleep(Duration::from_millis(4000));
}