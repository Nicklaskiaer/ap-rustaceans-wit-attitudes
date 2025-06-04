use std::time::Duration;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Fragment, Packet};
use crate::client::client_server_command::ClientServerCommand;
use crate::simulation_controller::simulation_controller::SimulationController;

pub fn run_tests(simulation_controller: &SimulationController) {
    // Wait a little
    std::thread::sleep(Duration::from_millis(1500));
    
    // drone_message_forward_test(simulation_controller);
    // drone_error_in_routing_test(simulation_controller);
    client_and_server_is_working_test(simulation_controller);
    client_send_message_test(simulation_controller);
}

fn drone_message_forward_test(simulation_controller: &SimulationController) {
    println!("Starting drone_message_forward_test, hops: 11, 21, 31, 32, 42");
    let msg = Packet::new_fragment(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![11, 21, 31, 32, 42],
        },
        1,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        },
    );

    //sends packet to D21
    if let Some((d21_sender, _)) = simulation_controller.get_packet_channels().get(&21) {
        d21_sender.send(msg.clone()).unwrap();
    }
    
    // Wait a little
    std::thread::sleep(Duration::from_millis(3500));
    println!("Completed drone_message_forward_test");
}

fn drone_error_in_routing_test(simulation_controller: &SimulationController) {
    println!("Starting drone_error_in_routing_test, hops: 11, 21, 31, 32, 41 (41 doesn't exist)");
    let msg = Packet::new_fragment(
        SourceRoutingHeader {
            hop_index: 1,
            hops: vec![11, 21, 31, 32, 41],
        },
        2,
        Fragment {
            fragment_index: 1,
            total_n_fragments: 1,
            length: 128,
            data: [1; 128],
        },
    );

    //sends packet to D21
    if let Some((d21_sender, _)) = simulation_controller.get_packet_channels().get(&21) {
        d21_sender.send(msg.clone()).unwrap();
    }

    // Wait a little
    std::thread::sleep(Duration::from_millis(3500));
    println!("Completed drone_error_in_routing_test");
}

fn client_send_message_test(simulation_controller: &SimulationController) {
    println!("Starting client_send_message_test, from C11 to S42");
    
    let message = "Qui Quo Quack".to_string();
    let message_id = 1000;
    let server_id = 41;

    //sends packet to C11
    if let Some((c11_sender, _)) = simulation_controller.get_clients().get(&11) {
        match c11_sender.send(ClientServerCommand::SendChatMessage(server_id, message)) {
            Ok(_) => {},
            Err(e) => println!("Failed to send chat message command: {:?}", e)
        }
    }

    // Wait a little
    std::thread::sleep(Duration::from_millis(3000));
    println!("Completed client_send_message_test");
}

fn client_and_server_is_working_test(simulation_controller: &SimulationController) {
    println!("Starting client_and_server_is_working_test for C11 and S42");
    let ack = Packet::new_ack(
        SourceRoutingHeader { hop_index: 0, hops: vec![] },
        0,
        0
    );

    //sends packet to C11
    if let Some((c11_sender, _)) = simulation_controller.get_packet_channels().get(&11) {
        match c11_sender.send(ack.clone()) {
            Ok(_) => {},
            Err(e) => println!("Failed to send packet: {:?}", e)
        }
    } else {
        println!("ERROR: Could not find channel for C11");
    }

    //sends packet to S42
    if let Some((s42_sender, _)) = simulation_controller.get_packet_channels().get(&42) {
        match s42_sender.send(ack.clone()) {
            Ok(_) => {},
            Err(e) => println!("Failed to send packet: {:?}", e)
        }
    } else {
        println!("ERROR: Could not find channel for C11");
    }

    // Wait a little
    std::thread::sleep(Duration::from_millis(3000));
    println!("Completed client_and_server_is_working_test");
}














