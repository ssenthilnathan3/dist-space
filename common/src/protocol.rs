use crate::proto::workspace::{OperationProto, SyncDocumentProto};
use bytes::{Buf, BufMut, BytesMut};
use prost::Message;
use std::io::Cursor;

// Our application enum
pub enum ServerMessage {
    Operation(OperationProto),
    SyncDocument(SyncDocumentProto),
}

impl ServerMessage {
    /// Serializes the inner Protobuf message and wraps it in a length-prefixed buffer with a type ID.
    /// Buffer format: [u32 length (of Type ID + Payload)][u8 type_id][...payload bytes...]
    pub fn encode(&self) -> Vec<u8> {
        let serialized_payload = match self {
            ServerMessage::Operation(op_proto) => op_proto.encode_to_vec(),
            ServerMessage::SyncDocument(sync_proto) => sync_proto.encode_to_vec(),
        };

        let type_id = match self {
            ServerMessage::Operation(_) => 1_u8,
            ServerMessage::SyncDocument(_) => 2_u8,
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

        // Read the total length (optional, as we already have the full slice,
        // but good practice if reading from a stream incrementally)
        let _total_length = cursor.get_u32();

        // Read the type ID discriminator
        let type_id = cursor.get_u8();

        // The remaining bytes are the actual protobuf payload
        // We create a slice of the bytes starting from the cursor's current position
        let payload_slice = &frame_bytes[cursor.position() as usize..];

        match type_id {
            1 => {
                // Decode as OperationProto
                let proto = OperationProto::decode(payload_slice)?;
                Ok(ServerMessage::Operation(proto))
            }
            2 => {
                // Decode as SyncDocumentProto
                let proto = SyncDocumentProto::decode(payload_slice)?;
                Ok(ServerMessage::SyncDocument(proto))
            }
            _ => Err(format!("Unknown message type ID: {}", type_id).into()),
        }
    }

    pub fn get_message_type_id(&self) -> u8 {
        match &self {
            ServerMessage::Operation(_) => 1_u8,
            ServerMessage::SyncDocument(_) => 2_u8,
        }
    }
}
