use crate::proto::space::{OperationProto, SyncDocumentProto};
use bytes::{Buf, BufMut, BytesMut};
use prost::Message;
use std::io::Cursor;

/// Server-to-client and client-to-server message types.
pub enum ServerMessage {
    /// An operation (edit) to be applied.
    Operation(OperationProto),
    /// Full document sync (state snapshot).
    SyncDocument(SyncDocumentProto),
    /// Ping message - sent by server to check client liveness.
    Ping(u64),
    /// Pong message - response to Ping with the same sequence number.
    Pong(u64),
}

/// Message type IDs for protocol encoding.
const MSG_TYPE_OPERATION: u8 = 1;
const MSG_TYPE_SYNC_DOCUMENT: u8 = 2;
const MSG_TYPE_PING: u8 = 3;
const MSG_TYPE_PONG: u8 = 4;

impl ServerMessage {
    /// Serializes the inner Protobuf message and wraps it in a length-prefixed buffer with a type ID.
    /// Buffer format: [u32 length (of Type ID + Payload)][u8 type_id][...payload bytes...]
    pub fn encode(&self) -> Vec<u8> {
        let (type_id, serialized_payload) = match self {
            ServerMessage::Operation(op_proto) => (MSG_TYPE_OPERATION, op_proto.encode_to_vec()),
            ServerMessage::SyncDocument(sync_proto) => {
                (MSG_TYPE_SYNC_DOCUMENT, sync_proto.encode_to_vec())
            }
            ServerMessage::Ping(seq) => {
                // Encode as 8 bytes (u64)
                (MSG_TYPE_PING, seq.to_be_bytes().to_vec())
            }
            ServerMessage::Pong(seq) => {
                // Encode as 8 bytes (u64)
                (MSG_TYPE_PONG, seq.to_be_bytes().to_vec())
            }
        };

        // Total length includes the 1-byte type_id + the payload length
        let total_payload_length = (serialized_payload.len() + 1) as u32;

        // Buffer capacity needed: 4 bytes for the length prefix + the data itself
        let mut buffer = BytesMut::with_capacity(4 + total_payload_length as usize);

        buffer.put_u32(total_payload_length); // Write the length
        buffer.put_u8(type_id); // Write the type ID
        buffer.put(serialized_payload.as_slice()); // Write the data

        buffer.freeze().to_vec()
    }

    /// Deserializes a raw byte slice (from a Frame payload) into a ServerMessage enum variant.
    /// This function reads the type ID to know which protobuf struct to decode into.
    pub fn decode(frame_bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        // We use a Cursor to track our position as we read through the bytes
        let mut cursor = Cursor::new(frame_bytes);

        // Read the total length
        let _total_length = cursor.get_u32();

        // Read the type ID discriminator
        let type_id = cursor.get_u8();

        // The remaining bytes are the actual protobuf payload
        // We create a slice of the bytes starting from the cursor's current position
        let payload_slice = &frame_bytes[cursor.position() as usize..];

        match type_id {
            MSG_TYPE_OPERATION => {
                // Decode as OperationProto
                let proto = OperationProto::decode(payload_slice)?;
                Ok(ServerMessage::Operation(proto))
            }
            MSG_TYPE_SYNC_DOCUMENT => {
                // Decode as SyncDocumentProto
                let proto = SyncDocumentProto::decode(payload_slice)?;
                Ok(ServerMessage::SyncDocument(proto))
            }
            MSG_TYPE_PING => {
                // Decode sequence number
                if payload_slice.len() < 8 {
                    return Err("Ping payload too short".into());
                }
                let seq = u64::from_be_bytes(
                    payload_slice[..8]
                        .try_into()
                        .map_err(|_| "Invalid ping payload")?,
                );
                Ok(ServerMessage::Ping(seq))
            }
            MSG_TYPE_PONG => {
                // Decode sequence number
                if payload_slice.len() < 8 {
                    return Err("Pong payload too short".into());
                }
                let seq = u64::from_be_bytes(
                    payload_slice[..8]
                        .try_into()
                        .map_err(|_| "Invalid pong payload")?,
                );
                Ok(ServerMessage::Pong(seq))
            }
            _ => Err(format!("Unknown message type ID: {}", type_id).into()),
        }
    }

    pub fn get_message_type_id(&self) -> u8 {
        match &self {
            ServerMessage::Operation(_) => MSG_TYPE_OPERATION,
            ServerMessage::SyncDocument(_) => MSG_TYPE_SYNC_DOCUMENT,
            ServerMessage::Ping(_) => MSG_TYPE_PING,
            ServerMessage::Pong(_) => MSG_TYPE_PONG,
        }
    }
}
