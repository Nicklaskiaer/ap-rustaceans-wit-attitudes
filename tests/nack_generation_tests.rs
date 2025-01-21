use ap_project_rustaceans_wit_attitudes::drone::MyDrone;
use crossbeam_channel::unbounded;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_2024::controller::DroneEvent;
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Fragment, Nack, NackType, Packet};
const TIMEOUT: Duration = Duration::from_millis(400);
#[cfg(test)]
#[test]
pub fn nack_unexpected_recipient_test() {
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    let (_d12_command_send, d12_command_recv) = unbounded();
    //Drone events
    let (d11_event_send, _d11_event_recv) = unbounded();
    let (d12_event_send, d12_event_recv) = unbounded();
    //Creates the Drone 11
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
    //Creates the Drone 12
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
    //Test Message
    let msg = Packet::new_fragment(
        SourceRoutingHeader {
            hop_index: 2,
            hops: vec![1, 11, 13, 21],
        },
        1,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        },
    );

    // "12" sends packet to 11
    d12_send.send(msg.clone()).unwrap();
    //Expected Response
    let nack_packet = Packet::new_nack(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![13, 11, 1],
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type: NackType::UnexpectedRecipient(12),
        },
    );

    // D11 receives nack_packet from D12
    assert_eq!(d11_recv.recv_timeout(TIMEOUT).unwrap(), nack_packet);
    // SC listen for event from the drone
    assert_eq!(
        d12_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(nack_packet)
    );
}

#[cfg(test)]
#[test]
pub fn nack_destination_is_drone_test() {
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    let (_d12_command_send, d12_command_recv) = unbounded();
    //Drone events
    let (d11_event_send, _d11_event_recv) = unbounded();
    let (d12_event_send, d12_event_recv) = unbounded();

    //Creates the Drone 11
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
    //Test Fragment
    let mut msg = Packet::new_fragment(
        SourceRoutingHeader {
            hop_index: 2,
            hops: vec![1, 11, 12],
        },
        1,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        },
    );
    //Supposed response
    let nack_packet = Packet::new_nack(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![12, 11,1],
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type: NackType::DestinationIsDrone,
        },
    );

    //D12 sends packet to D11
    d12_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 2;

    //D11 receives packet from D12
    assert_eq!(d11_recv.recv_timeout(TIMEOUT).unwrap(), nack_packet);
    //SC listen for event from the drone
    assert_eq!(
        d12_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(nack_packet)
    );
}

#[cfg(test)]
#[test]
pub fn nack_error_in_routing_test() {
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
    //Creates Drone 11
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
    //Test Fragment
    let mut msg = Packet::new_fragment(
        SourceRoutingHeader {
            hop_index: 2,
            hops: vec![1, 11, 12, 15],
        },
        1,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        },
    );
    //Expected Response
    let nack_packet = Packet::new_nack(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![12, 11,1],
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type: NackType::ErrorInRouting(15),
        },
    );

    //D12 sends packet to D11
    d12_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 2;

    //D12 receives packet from D11
    assert_eq!(d11_recv.recv_timeout(TIMEOUT).unwrap(), nack_packet);
    //SC listen for event from the drone
    assert_eq!(
        d12_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(nack_packet)
    );
}

#[cfg(test)]
#[test]
pub fn nack_dropped_test() {
    //Client
    let (c_send, _c_recv) = unbounded();
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //Drone 12
    let (d12_send, _d12_recv) = unbounded::<Packet>();
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
        HashMap::from([(12, d12_send.clone()), (1,c_send.clone())]),
        1.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Test Fragment
    let mut msg = Packet::new_fragment(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 11, 12],
        },
        1,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        },
    );
    //Expected Response
    let  nack_packet = Packet::new_nack(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![11,1],
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type: NackType::Dropped,
        },
    );

    //D11 sends packet to D12
    d11_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 1;

    //Client receives Nack from Drone 11
    assert_eq!(_c_recv.recv_timeout(TIMEOUT).unwrap(), nack_packet);
    //SC listen for event from the drone
    assert_eq!(
        d11_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketDropped(msg)
    );
}