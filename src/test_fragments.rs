use crossbeam_channel::unbounded;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_2024::controller::DroneEvent;
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{FloodRequest, Fragment, Nack, NackType, NodeType, Packet, PacketType};

/* THE FOLLOWING TESTS CHECKS IF YOUR DRONE IS HANDLING CORRECTLY PACKETS (FRAGMENT) */

const TIMEOUT: Duration = Duration::from_millis(1000);

/// Creates a sample packet for testing purposes. For convenience, using 1-10 for clients, 11-20 for drones and 21-30 for servers
fn create_sample_packet() -> Packet {
    Packet::new_fragment(
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
    )
}

/// This function is used to test the packet forward functionality of a drone.
/// The assert consists in checking if the "client" and "SC" receive the correct packet.
pub fn generic_fragment_forward<T: Drone + Send + 'static>() {
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d2_send, d2_recv) = unbounded::<Packet>();
    // SC commands
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    let mut drone = T::new(
        11,
        d_event_send,
        d_command_recv,
        d_recv.clone(),
        HashMap::from([(12, d2_send.clone())]),
        0.0,
    );
    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    let mut msg = create_sample_packet();

    // "Client" sends packet to d
    d_send.send(msg.clone()).unwrap();
    msg.routing_header.hop_index = 2; //TODO: aaaaaaaaaaaaaa (2)

    // d2 receives packet from d1
    assert_eq!(d2_recv.recv_timeout(TIMEOUT).unwrap(), msg);

    // SC listen for event from the drone
    assert_eq!(
        d_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketSent(msg)
    );
}

/// Checks if the packet is dropped by one drone. The assert consists in checking if the "client" and "SC" receive the correct packet.
pub fn generic_fragment_drop<T: Drone + Send + 'static>() {
    // Client 1
    let (c_send, c_recv) = unbounded();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d2_send, _d2_recv) = unbounded();
    // SC commands
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    let mut drone = T::new(
        11,
        d_event_send,
        d_command_recv,
        d_recv,
        HashMap::from([(12, d2_send.clone()), (1, c_send.clone())]),
        1.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    let msg = create_sample_packet();

    // "Client" sends packet to the drone
    d_send.send(msg.clone()).unwrap();

    let nack_packet = Packet::new_nack(
        SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 11, 12, 21],
        },
        1,
        Nack {
            fragment_index: 1,
            nack_type: NackType::Dropped,
        },
    );

    // Client listens for packet from the drone (Dropped Nack)
    assert_eq!(c_recv.recv_timeout(TIMEOUT).unwrap(), nack_packet);

    // SC listen for event from the drone
    assert_eq!(
        d_event_recv.recv_timeout(TIMEOUT).unwrap(),
        DroneEvent::PacketDropped(nack_packet)
    );
}

/// Checks if the packet is dropped by the second drone. The first drone has 0% PDR and the second one 100% PDR, otherwise the test will fail sometimes.
/// The assert is checking only the NACK received by the client (It does not care about the SC events).
pub fn generic_chain_fragment_drop<T: Drone + Send + 'static>() {
    // Client 1 channels
    let (c_send, c_recv) = unbounded();
    // Server 21 channels
    let (s_send, _s_recv) = unbounded();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();
    // SC - needed to not make the drone crash / send DroneEvents
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, _d_event_recv) = unbounded();

    // Drone 11
    let mut drone = T::new(
        11,
        d_event_send.clone(),
        d_command_recv.clone(),
        d_recv,
        HashMap::from([(12, d12_send.clone()), (1, c_send.clone())]),
        0.0,
    );
    // Drone 12
    let mut drone2 = T::new(
        12,
        d_event_send.clone(),
        d_command_recv.clone(),
        d12_recv,
        HashMap::from([(11, d_send.clone()), (21, s_send.clone())]),
        1.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    thread::spawn(move || {
        drone2.run();
    });

    let msg = create_sample_packet();

    // "Client" sends packet to the drone
    d_send.send(msg.clone()).unwrap();

    // Client receives an NACK originated from 'd2'
    assert_eq!(
        c_recv.recv_timeout(TIMEOUT).unwrap(),
        Packet {
            pack_type: PacketType::Nack(Nack {
                fragment_index: 1,
                nack_type: NackType::Dropped,
            }),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: vec![1, 11, 12, 21],
            },
            session_id: 1,
        }
    );
}

/// Checks if the packet can reach its destination. Both drones must have 0% PDR, otherwise the test will fail sometimes.
/// The assert is checking only the ACK received by the client (It does not care about the SC events).
pub fn generic_chain_fragment_ack<T: Drone + Send + 'static>() {
    // Client 1
    let (c_send, c_recv) = unbounded();
    // Server 21
    let (s_send, s_recv) = unbounded();
    // Drone 11
    let (d_send, d_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();
    // SC - needed to not make the drone crash
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, _d_event_recv) = unbounded();

    // Drone 11
    let mut drone = T::new(
        11,
        d_event_send.clone(),
        d_command_recv.clone(),
        d_recv,
        HashMap::from([(12, d12_send.clone()), (1, c_send.clone())]),
        0.0,
    );
    // Drone 12
    let mut drone2 = T::new(
        12,
        d_event_send.clone(),
        d_command_recv.clone(),
        d12_recv,
        HashMap::from([(11, d_send.clone()), (21, s_send.clone())]),
        0.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone.run();
    });

    thread::spawn(move || {
        drone2.run();
    });

    let mut msg = create_sample_packet();

    // "Client" sends packet to the drone
    d_send.send(msg.clone()).unwrap();

    msg.routing_header.hop_index = 3;

    // Server receives the fragment
    assert_eq!(s_recv.recv_timeout(TIMEOUT).unwrap(), msg);

    // Server sends an ACK
    d12_send
        .send(Packet::new_ack(
            SourceRoutingHeader {
                hop_index: 2,
                hops: vec![1, 11, 12, 21],
            },
            1,
            1,
        ))
        .unwrap();

    // Client receives an ACK originated from 's'
    assert_eq!(
        c_recv.recv_timeout(TIMEOUT).unwrap(),
        Packet::new_ack(
            SourceRoutingHeader {
                hop_index: 0,
                hops: vec![1, 11, 12, 21],
            },
            1,
            1,
        )
    );
}

/// Checks if the FloodRequest is correctly handled and forwarded by the drone.
/// The assert consists in checking if the neighbors receive the correct FloodRequest packet.
pub fn generic_flood_request<T: Drone + Send + 'static>() {
    // Client 1
    let (c1_send, _c1_recv) = unbounded();
    // Drone 11
    let (d11_send, d11_recv) = unbounded();
    // Drone 12
    let (d12_send, d12_recv) = unbounded();
    // Drone 21
    let (d21_send, d21_recv) = unbounded();
    // SC commands
    let (_d_command_send, d_command_recv) = unbounded();
    let (d_event_send, d_event_recv) = unbounded();

    // Drone 11
    let mut drone11 = T::new(
        11,
        d_event_send.clone(),
        d_command_recv.clone(),
        d11_recv.clone(),
        HashMap::from([
            (21, d21_send.clone()),
            (12, d12_send.clone()),
            (1, c1_send.clone()),
        ]),
        0.0,
    );

    // Drone 12
    let mut drone12 = T::new(
        12,
        d_event_send.clone(),
        d_command_recv.clone(),
        d12_recv.clone(),
        HashMap::from([
            (21, d21_send.clone()),
            (11, d12_send.clone()),
            (1, c1_send.clone()),
        ]),
        0.0,
    );

    // Drone 21
    let mut drone21 = T::new(
        21,
        d_event_send.clone(),
        d_command_recv.clone(),
        d21_recv.clone(),
        HashMap::from([
            (11, d11_send.clone()),
            (12, d12_send.clone()),
            (1, c1_send.clone()),
        ]),
        0.0,
    );

    // Spawn the drone's run method in a separate thread
    thread::spawn(move || {
        drone11.run();
    });

    thread::spawn(move || {
        drone12.run();
    });

    thread::spawn(move || {
        drone21.run();
    });

    let flood_request = Packet::new_flood_request(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1],
        },
        1,
        FloodRequest {
            flood_id: 99,
            initiator_id: 1,
            path_trace: vec![(1, NodeType::Client)],
        },
    );

    // "Client" sends FloodRequest packet to the drone
    d11_send.send(flood_request.clone()).unwrap();
    // d12_send.send(flood_request.clone()).unwrap();

    while let Ok(event) = d_event_recv.recv_timeout(TIMEOUT) {
        println!("SC Command {:?}", event);
    }
    // while let Ok(event) = d_event_recv.recv_timeout(TIMEOUT) {
    //     println!("SC Event {:?}", event);
    // }
    // while let Ok(event) = c1_recv.recv_timeout(TIMEOUT) {
    //     println!("c1 {:?}", event);
    // }
    // while let Ok(event) = d11_recv.recv_timeout(TIMEOUT) {
    //     println!("d11 {:?}", event);
    // }
    // while let Ok(event) = d12_recv.recv_timeout(TIMEOUT) {
    //     println!("d12 {:?}", event);
    // }
    // while let Ok(event) = d21_recv.recv_timeout(TIMEOUT) {
    //     println!("d21 {:?}", event);
    // }

    // let mut expected_flood_request = flood_request.clone();
    // expected_flood_request.routing_header.hop_index = 2;
    // expected_flood_request.pack_type = PacketType::FloodRequest(FloodRequest {
    //     flood_id: 1,
    //     initiator_id: 1,
    //     path_trace: vec![(1, NodeType::Client), (11, NodeType::Drone)],
    // });
    //
    // // Drone 12 receives the FloodRequest from Drone 11
    // assert_eq!(d2_recv.recv_timeout(TIMEOUT).unwrap(), expected_flood_request);
    //
    // // SC listen for event from the drone
    // assert_eq!(
    //     d_event_recv.recv_timeout(TIMEOUT).unwrap(),
    //     DroneEvent::PacketSent(expected_flood_request)
    // );
}
