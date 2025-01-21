use ap_project_rustaceans_wit_attitudes::drone::MyDrone;
use crossbeam_channel::unbounded;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_2024::controller::DroneEvent;
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Nack, NackType, Packet};
const TIMEOUT: Duration = Duration::from_millis(400);

#[cfg(test)]
#[test]
pub fn drone_event_controller_shortcut_test() {
    //Client
    let (c_send, _c_recv) = unbounded();
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
        HashMap::from([(1,c_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Test Nack
    let nack_packet = Packet::new_nack(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![11, 12, 1],
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type: NackType::DestinationIsDrone,
        },
    );

    //D11 sends packet to D12
    d11_send.send(nack_packet.clone()).unwrap();

    //SC listen for event from the drone
    assert_eq!(
        d11_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::ControllerShortcut(nack_packet)
    );
}