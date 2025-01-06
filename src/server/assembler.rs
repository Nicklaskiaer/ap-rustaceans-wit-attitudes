use crossbeam_channel::{select_biased, unbounded, Receiver, SendError, Sender};
use wg_2024::packet::{Fragment, Packet, PacketType};

pub struct Assembler {
    pub session_id: u64,
    pub packet_send: Sender<Packet>,
    pub packet_recv: Receiver<Packet>,
    pub data: Vec<u8>,
}

impl Assembler {
    pub fn new(
        session_id: u64,
        packet_send: Sender<Packet>,
        packet_recv: Receiver<Packet>,
    ) -> Self {
        Self {
            session_id,
            packet_send,
            packet_recv,
            data: Vec::new(),
        }
    }

    pub fn assemble(&mut self) -> Result<Vec<u8>, String> {
        loop {
            let packet = self.packet_recv.recv().map_err(|e| e.to_string())?;
            if packet.session_id != self.session_id {
                return Err("Session ID mismatch".to_string());
            }
            match packet.pack_type {
                PacketType::MsgFragment(fragment) => {
                    self.data.extend(fragment.data);
                    if fragment.fragment_index == fragment.total_n_fragments - 1 {
                        return Ok(self.data.clone());
                    }
                }
                _ => return Err("Unexpected packet type".to_string()),
            }
        }
    }
}
