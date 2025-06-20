use crossbeam_channel::{select_biased, Receiver, Sender};
use wg_2024::packet::{Packet, PacketType};

pub struct DataAssembly {
    session_id: u64,
    data: Vec<u8>,
    total_fragments: u64,
    current_fragment_index: u64,
}

pub struct Assembler {
    pub assemblies: Vec<DataAssembly>,
    pub packet_send: Sender<Packet>,
    pub packet_recv: Receiver<Packet>,
    pub result_send: Sender<Vec<u8>>,
    pub result_recv: Receiver<Vec<u8>>,
}

impl Assembler {
    pub fn new(
        assemblies: Vec<DataAssembly>,
        packet_send: Sender<Packet>,
        packet_recv: Receiver<Packet>,
        result_send: Sender<Vec<u8>>,
        result_recv: Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            assemblies,
            packet_send,
            packet_recv,
            result_send,
            result_recv,
        }
    }
    pub fn run(&mut self) {
        loop {
            select_biased! {
                recv(self.packet_recv) -> packet => {
                    debug!("Assembler received packet: {:?}", packet);
                    if let Ok(packet) = packet {
                        self.handle_packet(packet);
                    }
                    else {
                        debug!("ERROR: Assembler experienced an error while receiving packet");
                    }
                }
            }
        }
    }

    fn handle_packet(&mut self, packet: Packet) {
        debug!("Assembler handling fragment: ");

        let session_id = packet.session_id;

        match packet.pack_type {
            PacketType::MsgFragment(fragment) => {
                if fragment.total_n_fragments == 0 {
                    debug!("Received fragment with total_n_fragments == 0, ignoring");
                    return;
                }
                // If total_n_fragments is 1, we can directly send back the data
                if fragment.total_n_fragments == 1 {
                    match self.result_send.send(fragment.data.to_vec().clone()) {
                        Ok(_) => {
                            debug!(
                                "Assembled data sent successfully for session_id: {}",
                                session_id
                            );
                        }
                        Err(_e) => {
                            debug!(
                                "Failed to send single fragment data for session_id: {}: {}",
                                session_id, _e
                            );
                        }
                    }
                    return;
                }

                // Check if the fragment has an assembly in progress
                let assembly = self
                    .assemblies
                    .iter_mut()
                    .find(|a| a.session_id == session_id);

                if let Some(assembly) = assembly {
                    assembly.data.extend(fragment.data.clone());
                    assembly.current_fragment_index += 1;

                    if assembly.current_fragment_index == assembly.total_fragments {
                        // All fragments received, process the data
                        match self.result_send.send(assembly.data.clone()) {
                            Ok(_) => {
                                debug!(
                                    "Assembled data for session_id: {} sent successfully",
                                    session_id
                                );
                            }
                            Err(_e) => {
                                debug!(
                                    "Failed to send assembled data for session_id: {}: {}",
                                    session_id, _e
                                );
                            }
                        }
                    }
                } else {
                    // No assembly in progress, create a new one
                    let mut new_assembly = DataAssembly {
                        session_id,
                        data: Vec::new(),
                        total_fragments: fragment.total_n_fragments,
                        current_fragment_index: 1,
                    };
                    new_assembly.data.extend(fragment.data.clone());

                    // Check if this is the last fragment
                    if new_assembly.current_fragment_index == new_assembly.total_fragments {
                        // All fragments received, process the data
                        match self.result_send.send(new_assembly.data.clone()) {
                            Ok(_) => {
                                debug!(
                                    "Assembled data for session_id: {} sent successfully",
                                    session_id
                                );
                            }
                            Err(_e) => {
                                debug!(
                                    "Failed to send assembled data for session_id: {}: {}",
                                    session_id, _e
                                );
                            }
                        }
                    }

                    self.assemblies.push(new_assembly);
                    debug!("New assembly created for session_id: {}", session_id);
                }
            }
            _ => {
                debug!("Received non-fragment packet: {:?}", packet);
            }
        }
    }
}
