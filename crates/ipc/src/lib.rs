//! Windows Named Pipe IPC with length-prefixed MessagePack framing and SDDL security descriptors.

pub mod client;
pub mod codec;
pub mod error;
pub mod protocol;
pub mod security;
pub mod server;

pub use client::ReconnectingIpcClient;
pub use codec::MsgPackCodec;
pub use error::IpcError;
pub use protocol::{
    AGENT_IPC_PIPE_NAME, AgentStatus, BROWSER_HOST_PIPE_NAME, IpcEnvelope, IpcMessage,
    MAX_IPC_FRAME_SIZE, TRAY_IPC_PIPE_NAME,
};
pub use security::{DEFAULT_PIPE_SDDL, PipeSecurityAttributes};
pub use server::IpcServer;
