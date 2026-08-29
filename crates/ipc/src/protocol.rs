use core_types::{DualTimestamp, GlobalEventId, RawEvent};
use serde::{Deserialize, Serialize};

pub const AGENT_IPC_PIPE_NAME: &str = r"\\.\pipe\trajectory-agent-ipc";
pub const BROWSER_HOST_PIPE_NAME: &str = r"\\.\pipe\trajectory-browser-host";
pub const TRAY_IPC_PIPE_NAME: &str = r"\\.\pipe\trajectory-tray-ipc";

pub const MAX_IPC_FRAME_SIZE: usize = 64 * 1024 * 1024; // 64 MiB limit

/// Top-level envelope for all IPC messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpcEnvelope<T> {
    pub message_id: u64,
    pub timestamp: DualTimestamp,
    pub payload: T,
}

/// Messages exchanged between Agent, Supervisor, Tray, and Browser Host
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IpcMessage {
    // === Registration & Handshake ===
    RegisterAgent {
        machine_id: String,
        session_id: String,
        windows_session_id: u32,
        user_name: String,
        agent_version: String,
        pid: u32,
    },
    RegisterAgentAck {
        assigned_machine_id: String,
        active_policy_hash: String,
        next_global_event_id: GlobalEventId,
    },

    // === Heartbeat & Diagnostics ===
    Heartbeat {
        agent_status: AgentStatus,
        memory_usage_bytes: u64,
        cpu_usage_pct: f32,
        queue_depth: usize,
        events_captured_total: u64,
    },
    HeartbeatAck {
        server_connected: bool,
        uploader_active: bool,
        storage_healthy: bool,
    },

    // === Session Boundary & Spool Events ===
    SessionBoundarySignal {
        previous_session_id: String,
        new_session_id: String,
        event_count: u64,
    },
    DiskWatermarkAlert {
        disk_tier: u8, // 0 = Normal, 1 = Caution (70%), 2 = Warning (85%), 3 = Critical (92%)
        free_bytes: u64,
        total_bytes: u64,
    },

    // === Supervisor Commands to Agent ===
    ConfigUpdate {
        config_toml: String,
        version: u64,
    },
    CommandPauseCapture {
        reason: String,
    },
    CommandResumeCapture,
    CommandForceRotation,

    // === Browser Telemetry Stream ===
    BrowserDomEvent(Box<RawEvent>),
    BrowserEventBatch(Vec<RawEvent>),

    // === System & Tray Queries ===
    GetStatusRequest,
    GetStatusResponse {
        is_recording: bool,
        current_session_id: String,
        events_this_session: u64,
        disk_free_gb: f64,
        server_status: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    Starting,
    Recording,
    Paused,
    Degraded,
    Stopping,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_ipc_message_variants_roundtrip() {
        let messages = vec![
            IpcMessage::RegisterAgent {
                machine_id: "m1".to_string(),
                session_id: "s1".to_string(),
                windows_session_id: 1,
                user_name: "u1".to_string(),
                agent_version: "1.0".to_string(),
                pid: 1234,
            },
            IpcMessage::RegisterAgentAck {
                assigned_machine_id: "m1".to_string(),
                active_policy_hash: "hash".to_string(),
                next_global_event_id: GlobalEventId::new(1),
            },
            IpcMessage::Heartbeat {
                agent_status: AgentStatus::Recording,
                memory_usage_bytes: 1024,
                cpu_usage_pct: 1.5,
                queue_depth: 0,
                events_captured_total: 100,
            },
            IpcMessage::HeartbeatAck {
                server_connected: true,
                uploader_active: true,
                storage_healthy: true,
            },
            IpcMessage::SessionBoundarySignal {
                previous_session_id: "s0".to_string(),
                new_session_id: "s1".to_string(),
                event_count: 500,
            },
            IpcMessage::DiskWatermarkAlert {
                disk_tier: 1,
                free_bytes: 10_000_000,
                total_bytes: 100_000_000,
            },
            IpcMessage::ConfigUpdate {
                config_toml: "version = 1".to_string(),
                version: 1,
            },
            IpcMessage::CommandPauseCapture {
                reason: "Disk full".to_string(),
            },
            IpcMessage::CommandResumeCapture,
            IpcMessage::CommandForceRotation,
            IpcMessage::GetStatusRequest,
        ];

        for msg in messages {
            let serialized = rmp_serde::to_vec_named(&msg).unwrap();
            let deserialized: IpcMessage = rmp_serde::from_slice(&serialized).unwrap();
            assert_eq!(msg, deserialized);
        }
    }
}
