use std::{io::Write, net::TcpStream, sync::Arc, thread};

use common::Frame;
use crossbeam::channel::{Receiver, RecvError};
use uuid::Uuid;

pub struct Writer;

impl Writer {
    pub fn spawn_writer_thread(
        client_id: Uuid,
        mut stream: TcpStream,
        rx: Receiver<Arc<Frame>>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            Writer::write_frames(client_id, &mut stream, rx);
        })
    }

    pub fn write_frames(client_id: Uuid, stream: &mut TcpStream, rx: Receiver<Arc<Frame>>) {
        loop {
            match rx.recv() {
                Ok(frame) => {
                    let payload_length = frame.payload.len();

                    let prefix = (payload_length as u32).to_be_bytes();

                    if let Err(e) = stream.write_all(&prefix) {
                        eprintln!(
                            "[WRITE] Writer for {} exiting: write error (prefix) - {}",
                            client_id, e
                        );
                        return; // Exit function on write error
                    }

                    if let Err(e) = stream.write_all(&frame.payload) {
                        eprintln!(
                            "[WRITE] Writer for {} exiting: write error payload - {}",
                            client_id, e
                        );
                        return; // Exit function on write error
                    }

                    println!(
                        "[WRITE] wrote frame with prefix=4 bytes and payload of length {} to writer of {}",
                        payload_length, client_id,
                    );
                }

                Err(RecvError) => {
                    // Channel closed - exit the loop gracefully to flush
                    eprintln!(
                        "[WRITE] Writer for {} exiting: channel disconnected",
                        client_id
                    );
                    break; // Use break to exit the loop
                }
            }
        }

        match stream.flush() {
            Ok(()) => {
                println!("[WRITE] Write completed and flushed the stream")
            }
            Err(e) => {
                eprintln!(
                    "[WRITE] Writer for {} exiting: flush error - {}",
                    client_id, e
                );
            }
        }
    }
}
