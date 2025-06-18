use crossbeam_channel::{select_biased, Receiver, Sender};
use wg_2024::packet::{Packet, PacketType};

struct DataAssembly {
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
                        self.handle_fragment(packet);
                    }
                }
            }
        }
    }

    fn handle_fragment(&mut self, packet: Packet) {
        debug!("Assembler handling fragment: {:?}", fragment);
        let session_id = packet.session_id;

        match packet.pack_type {
            PacketType::MsgFragment(fragment) => {
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
                        self.result_send.send(assembly.data.clone());

                        debug!(
                            "All fragments received for session_id: {} assembled data sent, data",
                            session_id
                        );
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
                    self.assemblies.push(new_assembly);
                    debug!("New assembly created for session_id: {}", session_id);
                }
            }
            _ => {
                debug!("Received non-fragment packet: {:?}", packet);
                // Handle other packet types if necessary
            }
        }
    }
}
