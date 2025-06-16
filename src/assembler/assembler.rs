use core::panic;

use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use wg_2024::packet::{Fragment, Packet, PacketType};

pub struct Assembler {
    pub session_id: u64,
    pub packet_send: Sender<Packet>,
    pub packet_recv: Receiver<Packet>,
    pub result_send: Sender<Vec<u8>>,
    pub result_recv: Receiver<Vec<u8>>,
    pub data: Vec<u8>,
}

impl Assembler {
    pub fn new(
        session_id: u64,
        packet_send: Sender<Packet>,
        packet_recv: Receiver<Packet>,
        result_send: Sender<Vec<u8>>,
        result_recv: Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            session_id,
            packet_send,
            packet_recv,
            result_send,
            result_recv,
            data: Vec::new(),
        }
    }
    fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.packet_recv) -> packet => {
                    debug!("Assembler received packet: {:?}", packet);
                    if let Ok(packet) = packet {
                        match packet.pack_type {
                            PacketType::MsgFragment(fragment) => {
                                self.data.extend(fragment.data);
                                if fragment.fragment_index == fragment.total_n_fragments - 1 {
                                    match self.result_send.send(self.data.clone()){
                                        Ok(_) => {
                                            break;
                                        }
                                        Err(SendError(data)) => {
                                            debug!("Failed to send result: {:?}", data);
                                        }
                                    }
                                }
                                else {
                                    match self.packet_send.send(Packet::new_ack(packet.routing_header, self.session_id, fragment.fragment_index)){
                                        Ok(_) => {}
                                        Err(SendError(packet)) => {
                                            debug!("Failed to send ack packet: {:?}", packet);
                                        }
                                    }
                                }
                            }
                            _ => {
                                debug!("Assembler received non-fragment packet: {:?}", packet);
                            }
                        }
                    }
                }
            }
        }
    }
}
