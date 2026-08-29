//! Trajectory Browser Native Messaging Host binary.
//! Bridges Chrome / Edge Manifest V3 Native Messaging stdio <-> Windows Named Pipe IPC.

use diagnostics::{DiagnosticsConfig, init_diagnostics};
use ipc::{IpcMessage, ReconnectingIpcClient};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{error, info, warn};

/// Browser-host sequences are source-local. The `GlobalEventId(0)` sentinel
/// is replaced by capture-agent's durable allocator before publication.
fn build_ipc_event(
    sequence: &AtomicU64,
    dom_event: browser_events::BrowserDomEvent,
) -> core_types::event::RawEvent {
    let source_sequence = sequence.fetch_add(1, Ordering::Relaxed);
    dom_event.to_unassigned_raw_event(source_sequence, "BROWSER_HOST", 1, "user")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_ingress_uses_monotonic_source_sequence_and_unassigned_global_id() {
        let sequence = AtomicU64::new(1);
        let dom_event = browser_events::BrowserDomEvent {
            tab_id: 1,
            url: "https://example.com/".to_string(),
            page_title: "Example".to_string(),
            event_type: "NAVIGATION_COMMITTED".to_string(),
            tag: "document".to_string(),
            role: None,
            visible_text: None,
            aria_label: None,
            element_id: None,
            class_name: None,
            href: None,
            placeholder: None,
            input_type: None,
            value_length: None,
            value: None,
            css_selector: None,
            xpath: None,
            timestamp_ms: 1,
            is_password: false,
            mutation_info: None,
        };

        let first = build_ipc_event(&sequence, dom_event.clone());
        let second = build_ipc_event(&sequence, dom_event);

        assert_eq!(first.event_id, 1);
        assert_eq!(second.event_id, 2);
        assert_eq!(
            first.global_event_id,
            Some(core_types::id::GlobalEventId::new(0))
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = init_diagnostics(&DiagnosticsConfig::default());
    info!("Trajectory Browser Host starting...");

    let pipe_name = r"\\.\pipe\trajectory-agent-ipc";
    let (send_tx, send_rx) = tokio::sync::mpsc::channel(100);
    let (recv_tx, _recv_rx) = tokio::sync::mpsc::channel(100);
    let cancel_token = tokio_util::sync::CancellationToken::new();

    let client = ReconnectingIpcClient::new(pipe_name, send_rx, recv_tx, cancel_token.clone());
    tokio::spawn(async move {
        client.run().await;
    });

    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    let mut len_buf = [0u8; 4];
    let source_sequence = AtomicU64::new(1);

    loop {
        // Read 4-byte native length prefix
        if stdin.read_exact(&mut len_buf).is_err() {
            break; // EOF or parent browser process closed
        }

        let msg_len = u32::from_ne_bytes(len_buf) as usize;
        if msg_len == 0 || msg_len > 10 * 1024 * 1024 {
            error!("Invalid Native Messaging payload length: {}", msg_len);
            break;
        }

        let mut payload_buf = vec![0u8; msg_len];
        if stdin.read_exact(&mut payload_buf).is_err() {
            break;
        }

        match std::str::from_utf8(&payload_buf) {
            Ok(json_str) => match serde_json::from_str::<browser_events::BrowserDomEvent>(json_str)
            {
                Ok(dom_event) => {
                    let raw = build_ipc_event(&source_sequence, dom_event);
                    let ipc_msg = IpcMessage::BrowserDomEvent(Box::new(raw));
                    if send_tx.send(ipc_msg).await.is_err() {
                        error!("Browser event IPC receiver closed; stopping host");
                        break;
                    }
                }
                Err(err) => warn!(error = %err, "Rejected malformed browser event payload"),
            },
            Err(err) => warn!(error = %err, "Rejected non-UTF-8 browser event payload"),
        }

        // Write 4-byte native length prefixed JSON response back to browser extension
        let response = b"{\"status\":\"received\"}";
        let resp_len = (response.len() as u32).to_ne_bytes();
        if stdout.write_all(&resp_len).is_err()
            || stdout.write_all(response).is_err()
            || stdout.flush().is_err()
        {
            break;
        }
    }

    info!("Trajectory Browser Host exited cleanly.");
    Ok(())
}
