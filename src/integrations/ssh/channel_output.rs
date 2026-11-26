use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read};
use std::sync::mpsc::{Receiver, TryRecvError};

/// Non-blocking `Read` over chunks pushed by the async side: `WouldBlock` when idle, `Ok(0)` once closed.
pub struct ChannelOutput {
    chunks: Receiver<Vec<u8>>,
    pending: VecDeque<u8>,
}

impl ChannelOutput {
    pub fn new(chunks: Receiver<Vec<u8>>, already_received: Vec<u8>) -> Self {
        Self { chunks, pending: already_received.into() }
    }
}

impl Read for ChannelOutput {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pending.is_empty() {
            match self.chunks.try_recv() {
                Ok(chunk) => self.pending.extend(chunk),
                Err(TryRecvError::Empty) => return Err(ErrorKind::WouldBlock.into()),
                Err(TryRecvError::Disconnected) => return Ok(0),
            }
        }
        self.pending.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn would_block_while_no_chunk_has_arrived() {
        let (_sender, receiver) = channel();
        let mut output = ChannelOutput::new(receiver, Vec::new());
        assert_eq!(output.read(&mut [0; 4]).unwrap_err().kind(), ErrorKind::WouldBlock);
    }

    #[test]
    fn returns_already_received_bytes_before_new_chunks() {
        let (sender, receiver) = channel();
        sender.send(b"cd".to_vec()).unwrap();
        let mut output = ChannelOutput::new(receiver, b"ab".to_vec());
        let mut collected = Vec::new();
        output.read_to_end(&mut collected).unwrap_err();
        assert_eq!(collected, b"abcd");
    }

    #[test]
    fn splits_a_chunk_larger_than_the_buffer() {
        let (sender, receiver) = channel();
        sender.send(b"hello".to_vec()).unwrap();
        let mut output = ChannelOutput::new(receiver, Vec::new());
        let mut buf = [0; 3];
        assert_eq!(output.read(&mut buf).unwrap(), 3);
        assert_eq!(output.read(&mut buf).unwrap(), 2);
        assert_eq!(&buf[..2], b"lo");
    }

    #[test]
    fn reports_end_of_stream_once_the_channel_closed() {
        let (sender, receiver) = channel();
        sender.send(b"bye".to_vec()).unwrap();
        drop(sender);
        let mut output = ChannelOutput::new(receiver, Vec::new());
        assert_eq!(output.read(&mut [0; 8]).unwrap(), 3);
        assert_eq!(output.read(&mut [0; 8]).unwrap(), 0);
    }
}
