use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use ipc::IpcMessage;

pub struct MockNamedPipePair {
    pub client_stream: DuplexStream,
    pub server_stream: DuplexStream,
}

impl MockNamedPipePair {
    pub fn new(buffer_size: usize) -> Self {
        let (client_stream, server_stream) = tokio::io::duplex(buffer_size);
        Self {
            client_stream,
            server_stream,
        }
    }

    /// Write length-prefixed MessagePack payload
    pub async fn write_message(stream: &mut DuplexStream, msg: &IpcMessage) -> Result<(), String> {
        let payload = rmp_serde::to_vec_named(msg).map_err(|e| format!("MsgPack serialize error: {e}"))?;
        let len = payload.len() as u32;
        stream.write_all(&len.to_be_bytes()).await.map_err(|e| e.to_string())?;
        stream.write_all(&payload).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read length-prefixed MessagePack payload
    pub async fn read_message(stream: &mut DuplexStream) -> Result<IpcMessage, String> {
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await.map_err(|e| e.to_string())?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
        
        let msg = rmp_serde::from_slice(&buf).map_err(|e| format!("MsgPack deserialize error: {e}"))?;
        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_named_pipe_framing_roundtrip() {
        let mut pair = MockNamedPipePair::new(4096);
        let msg = IpcMessage::SessionBoundarySignal {
            previous_session_id: "s1".to_string(),
            new_session_id: "s2".to_string(),
            event_count: 50,
        };

        MockNamedPipePair::write_message(&mut pair.client_stream, &msg).await.unwrap();
        let received = MockNamedPipePair::read_message(&mut pair.server_stream).await.unwrap();
        assert_eq!(msg, received);
    }
}
