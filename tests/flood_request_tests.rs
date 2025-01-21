use ap_project_rustaceans_wit_attitudes::drone::MyDrone;
use crossbeam_channel::unbounded;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{FloodRequest, FloodResponse, NodeType, Packet};
const TIMEOUT: Duration = Duration::from_millis(400);

#[cfg(test)]
#[test]
pub fn flood_response_end_in_drone_test() {
    //Client
    let (c_send, _c_recv) = unbounded();
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
        HashMap::from([(12, d12_send.clone()), (1,c_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Creates Drone 12
    let mut drone2 = MyDrone::new(
        12,
        d12_event_send,
        d12_command_recv,
        d12_recv.clone(),
        HashMap::from([(11, d11_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone2.run();
    });
    //Test Packet
    let mut flood_request = Packet::new_flood_request(
        SourceRoutingHeader {
            hop_index: 2,
            hops: vec![1, 11, 12],
        },
        1,
        FloodRequest{
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec!((1, NodeType::Client), (11, NodeType::Drone))
        }
    );
    //Expected Response
    let flood_response = Packet::new_flood_response(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![12,11,1],
        },
        1,
        FloodResponse{
            flood_id: 1,
            path_trace: vec!((1, NodeType::Client), (11, NodeType::Drone), (12, NodeType::Drone))
        }
    );

    //D12 sends packet to D11
    d12_send.send(flood_request.clone()).unwrap();
    flood_request.routing_header.hop_index = 1;

    //D11 receives packet from D11
    assert_eq!(d11_recv.recv_timeout(TIMEOUT).unwrap(), flood_response);
    //SC listen for event from the drone
    assert_eq!(
        d12_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(flood_response)
    );
}

#[cfg(test)]
#[test]
pub fn flood_request_already_received_test() {
    //Client
    let (c_send, _c_recv) = unbounded();
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
        HashMap::from([(12, d12_send.clone()), (1,c_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Creates Drone 12
    let mut drone2 = MyDrone::new(
        12,
        d12_event_send,
        d12_command_recv,
        d12_recv.clone(),
        HashMap::from([(11, d11_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone2.run();
    });
    //Expected Response
    let flood_response = Packet::new_flood_response(
        SourceRoutingHeader {
            hop_index: 2,
            hops: vec![12,11,1],
        },
        1,
        FloodResponse{
            flood_id: 1,
            path_trace: vec!((1, NodeType::Client), (11, NodeType::Drone), (12, NodeType::Drone))
        }
    );
    //Test Packet
    let mut flood_request = Packet::new_flood_request(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 11, 12],
        },
        1,
        FloodRequest{
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec!((1, NodeType::Client), (11, NodeType::Drone), (12, NodeType::Drone))
        }
    );
    //D12 sends packet to D11
    d12_send.send(flood_request.clone()).unwrap();
    flood_request.routing_header.hop_index = 1;
    //D11 receives packet from D12
    assert_eq!(d11_recv.recv_timeout(TIMEOUT).unwrap(), flood_response);
    //SC listen for event from the drone
    assert_eq!(
        d12_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(flood_response)
    );
}

#[cfg(test)]
#[test]
pub fn flood_request_forwarding_test() {
    //Client
    let (c_send, _c_recv) = unbounded();
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    let (_d12_command_send, _d12_command_recv) = unbounded::<DroneCommand>();
    //Drone Events
    let (d11_event_send, d11_event_recv) = unbounded();
    let (_d12_event_send, _d12_event_recv) = unbounded::<DroneEvent>();
    //Creates Drone 11
    let mut drone = MyDrone::new(
        11,
        d11_event_send,
        d11_command_recv,
        d11_recv.clone(),
        HashMap::from([(12, d12_send.clone()), (1,c_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Test Packet
    let flood_request = Packet::new_flood_request(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1,11],
        },
        1,
        FloodRequest{
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec!((1, NodeType::Client))
        }
    );
    //D11 sends packet to D12
    d11_send.send(flood_request.clone()).unwrap();
    //Expected Response
    let mut flood_request = Packet::new_flood_request(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1,11],
        },
        1,
        FloodRequest{
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec!((1, NodeType::Client),(11, NodeType::Drone))
        }
    );
    flood_request.routing_header.hop_index = 1;
    //D12 receives packet from D11
    assert_eq!(d12_recv.recv_timeout(TIMEOUT).unwrap(), flood_request);
    //SC listen for event from the drone
    assert_eq!(
        d11_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(flood_request)
    );
}

/*#[cfg(test)]
#[test]
pub fn flood_response_end_in_drone_test1() {
    //Client
    let (c_send, c_recv) = unbounded();
    //Drone 11
    let (d11_send, d11_recv) = unbounded();
    //Drone 12
    let (d12_send, d12_recv) = unbounded::<Packet>();
    //SC commands
    let (_d11_command_send, d11_command_recv) = unbounded();
    let (_d12_command_send, d12_command_recv) = unbounded();
    //Drone Events
    let (d11_event_send, d11_event_recv) = unbounded();
    let (d12_event_send, d12_event_recv) = unbounded();
    //Creates Drone 11
    let mut drone = MyDrone::new(
        11,
        d11_event_send,
        d11_command_recv.clone(),
        d11_recv.clone(),
        HashMap::from([(12, d12_send.clone()), (1,c_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });
    //Creates Drone 12
    let mut drone2 = MyDrone::new(
        12,
        d12_event_send,
        d12_command_recv.clone(),
        d12_recv.clone(),
        HashMap::from([(11, d11_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone2.run();
    });
    //Test Packet
    let flood_request = Packet::new_flood_request(
        SourceRoutingHeader {
            hop_index: 2,
            hops: vec![1, 11, 12],
        },
        1,
        FloodRequest{
            flood_id: 1,
            initiator_id: 1,
            path_trace: vec!((1, NodeType::Client), (11, NodeType::Drone))
        }
    );

    //D12 sends packet to D11
    d12_send.send(flood_request).unwrap();

    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(20) {
            break;
        }
        if let Ok(packet) = c_recv.recv_timeout(Duration::from_millis(600)) {
            println!("{:?}", packet);
        }
        if let Ok(packet) = d11_recv.recv_timeout(Duration::from_millis(600)) {
            println!("{:?}", packet);
        }
        if let Ok(packet) = d12_recv.recv_timeout(Duration::from_millis(600)) {
            println!("{:?}", packet);
        }
        if let Ok(packet) = d11_command_recv.recv_timeout(Duration::from_millis(600)) {
            println!("{:?}", packet);
        }
        if let Ok(packet) = d12_command_recv.recv_timeout(Duration::from_millis(600)) {
            println!("{:?}", packet);
        }
        if let Ok(packet) = d11_event_recv.recv_timeout(Duration::from_millis(600)) {
            println!("{:?}", packet);
        }
        if let Ok(packet) = d12_event_recv.recv_timeout(Duration::from_millis(600)) {
            println!("{:?}", packet);
        }
    }
}
*/