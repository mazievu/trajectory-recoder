use bytes::{Buf, BufMut, BytesMut};
use serde::{de::DeserializeOwned, Serialize};
use tokio_util::codec::{Decoder, Encoder};
use crate::error::IpcError;
use crate::protocol::MAX_IPC_FRAME_SIZE;

/// Framed Codec: 4-byte big-endian length prefix + rmp-serde MessagePack payload.
pub struct MsgPackCodec<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Default for MsgPackCodec<T> {
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Clone for MsgPackCodec<T> {
    fn clone(&self) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Serialize> Encoder<T> for MsgPackCodec<T> {
    type Error = IpcError;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let serialized = rmp_serde::to_vec_named(&item)
            .map_err(|e| IpcError::SerializationError(e.to_string()))?;

        if serialized.len() > MAX_IPC_FRAME_SIZE {
            return Err(IpcError::FrameTooLarge {
                size: serialized.len(),
                max: MAX_IPC_FRAME_SIZE,
            });
        }

        dst.reserve(4 + serialized.len());
        dst.put_u32(serialized.len() as u32);
        dst.put_slice(&serialized);
        Ok(())
    }
}

impl<T: DeserializeOwned> Decoder for MsgPackCodec<T> {
    type Item = T;
    type Error = IpcError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let payload_len = u32::from_be_bytes(length_bytes) as usize;

        if payload_len > MAX_IPC_FRAME_SIZE {
            return Err(IpcError::FrameTooLarge {
                size: payload_len,
                max: MAX_IPC_FRAME_SIZE,
            });
        }

        if src.len() < 4 + payload_len {
            src.reserve((4 + payload_len) - src.len());
            return Ok(None);
        }

        src.advance(4);
        let payload = src.split_to(payload_len);
        let item: T = rmp_serde::from_slice(&payload)
            .map_err(|e| IpcError::DeserializationError(e.to_string()))?;

        Ok(Some(item))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::IpcMessage;

    #[test]
    fn test_codec_encode_decode_roundtrip() {
        let mut codec = MsgPackCodec::<IpcMessage>::default();
        let mut buf = BytesMut::new();

        let msg = IpcMessage::CommandResumeCapture;
        codec.encode(msg.clone(), &mut buf).unwrap();

        assert!(buf.len() > 4);
        let decoded = codec.decode(&mut buf).unwrap().expect("Decoded message");
        assert_eq!(msg, decoded);
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_codec_partial_fragment_decode() {
        let mut codec = MsgPackCodec::<IpcMessage>::default();
        let mut full_buf = BytesMut::new();

        let msg = IpcMessage::CommandResumeCapture;
        codec.encode(msg.clone(), &mut full_buf).unwrap();

        // Feed bytes one by one
        let mut stream_buf = BytesMut::new();
        for i in 0..full_buf.len() - 1 {
            stream_buf.put_u8(full_buf[i]);
            let res = codec.decode(&mut stream_buf).unwrap();
            assert!(res.is_none());
        }

        stream_buf.put_u8(*full_buf.last().unwrap());
        let res = codec.decode(&mut stream_buf).unwrap();
        assert_eq!(res, Some(msg));
    }
}
