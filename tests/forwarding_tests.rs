use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use crossbeam_channel::unbounded;
use wg_2024::controller::DroneEvent;
use wg_2024::drone::Drone;
use wg_2024::network::{SourceRoutingHeader};
use wg_2024::packet::{Nack, FloodResponse, Fragment, Packet, NackType, NodeType};
use ap_project_rustaceans_wit_attitudes::drone::MyDrone;
const TIMEOUT: Duration = Duration::from_millis(400);
#[cfg(test)]
#[test]
pub fn fragment_forwarding() {
    // Drone 11
    let (d11_send, d11_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    // SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    //Node Events
    let (d11_event_send, d11_event_recv) = unbounded();
    //Creates Drone11
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
    //Test Fragment
    let mut msg = Packet::new_fragment(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 11, 12, 21],
        },
        1,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        },
    );

    //D12 sends packet to D11
    d11_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 2;

    //D12 receives packet from D11
    assert_eq!(d12_recv.recv_timeout(TIMEOUT).unwrap(), msg);
    //SC listen for event from the drone
    assert_eq!(
        d11_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(msg)
    );
}

#[cfg(test)]
#[test]
pub fn ack_forwarding() {
    // Drone 11
    let (d11_send, d11_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    // SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    //Node Events
    let (d11_event_send, d11_event_recv) = unbounded();
    //Creates Drone11
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
    //Test Fragment
    let mut msg = Packet::new_ack(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 11, 12, 21],
        },
        1,
        1
    );

    //D12 sends packet to D11
    d11_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 2;

    //D12 receives packet from D11
    assert_eq!(d12_recv.recv_timeout(TIMEOUT).unwrap(), msg);
    //SC listen for event from the drone
    assert_eq!(
        d11_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(msg)
    );
}

#[cfg(test)]
#[test]
pub fn nack_forwarding_dropped() {
    // Drone 11
    let (d11_send, d11_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    // SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    //Node Events
    let (d11_event_send, d11_event_recv) = unbounded();
    //Creates Drone11
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
    let mut nack_packet = Packet::new_nack(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 11, 12, 21],
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type: NackType::Dropped,
        },
    );

    //D12 sends packet to D11
    d11_send.send(nack_packet.clone()).unwrap();
    nack_packet.routing_header.hop_index = 2;

    //D12 receives packet from D11
    assert_eq!(d12_recv.recv_timeout(TIMEOUT).unwrap(), nack_packet);
    //SC listen for event from the drone
    assert_eq!(
        d11_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(nack_packet)
    );
}

#[cfg(test)]
#[test]
pub fn nack_forwarding_DestinationIsDrone() {
    // Drone 11
    let (d11_send, d11_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    // SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    //Node Events
    let (d11_event_send, d11_event_recv) = unbounded();
    //Creates Drone11
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
    let mut nack_packet = Packet::new_nack(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 11, 12, 21],
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type: NackType::DestinationIsDrone,
        },
    );

    //D12 sends packet to D11
    d11_send.send(nack_packet.clone()).unwrap();
    nack_packet.routing_header.hop_index = 2;

    //D12 receives packet from D11
    assert_eq!(d12_recv.recv_timeout(TIMEOUT).unwrap(), nack_packet);
    //SC listen for event from the drone
    assert_eq!(
        d11_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(nack_packet)
    );
}

#[cfg(test)]
#[test]
pub fn flood_response_forwarding() {
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    let (_d12_command_send, d12_command_recv) = unbounded();
    //Drone Events
    let (d11_event_send, _d11_event_recv) = unbounded();
    let (d12_event_send, d12_event_recv) = unbounded();
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
    //Creates Drone 12
    let mut drone = MyDrone::new(
        12,
        d12_event_send,
        d12_command_recv,
        d12_recv.clone(),
        HashMap::from([(11, d11_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Test Flood Response
    let mut flood_response = Packet::new_flood_response(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![21, 12, 11, 1],
        },
        1,
        FloodResponse {
            flood_id: 1,
            path_trace: vec![(1, NodeType::Client), (11, NodeType::Drone), (12, NodeType::Drone), (21, NodeType::Server)]
        }
    );

    //D12 sends packet to D11
    d12_send.send(flood_response.clone()).unwrap();
    flood_response.routing_header.hop_index = 2;

    //D11 receives packet from D11
    assert_eq!(d11_recv.recv_timeout(TIMEOUT).unwrap(), flood_response);
    //SC listen for event from the drone
    assert_eq!(
        d12_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(flood_response)
    );
}