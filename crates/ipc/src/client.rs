use std::time::Duration;
use futures::{SinkExt, StreamExt};
use rand::Rng;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

use crate::codec::MsgPackCodec;
use crate::error::IpcError;
use crate::protocol::IpcMessage;

pub struct ReconnectingIpcClient {
    pipe_name: String,
    send_rx: mpsc::Receiver<IpcMessage>,
    receive_tx: mpsc::Sender<IpcMessage>,
    cancel_token: CancellationToken,
}

impl ReconnectingIpcClient {
    pub fn new(
        pipe_name: impl Into<String>,
        send_rx: mpsc::Receiver<IpcMessage>,
        receive_tx: mpsc::Sender<IpcMessage>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            pipe_name: pipe_name.into(),
            send_rx,
            receive_tx,
            cancel_token,
        }
    }

    pub async fn run(mut self) {
        let mut backoff = Duration::from_millis(100);
        let max_backoff = Duration::from_secs(10);

        while !self.cancel_token.is_cancelled() {
            debug!("Attempting connection to Named Pipe {}", self.pipe_name);
            match Self::connect_named_pipe(&self.pipe_name).await {
                Ok(client) => {
                    info!("Successfully connected to Named Pipe {}", self.pipe_name);
                    backoff = Duration::from_millis(100); // Reset backoff

                    let framed = Framed::new(client, MsgPackCodec::<IpcMessage>::default());
                    let (mut sink, mut stream) = framed.split();

                    loop {
                        tokio::select! {
                            _ = self.cancel_token.cancelled() => return,
                            send_item = self.send_rx.recv() => {
                                match send_item {
                                    Some(msg) => {
                                        if let Err(e) = sink.send(msg).await {
                                            warn!("Failed to send message over pipe: {e}");
                                            break; // Trigger reconnect
                                        }
                                    }
                                    None => return, // Send channel closed
                                }
                            }
                            recv_item = stream.next() => {
                                match recv_item {
                                    Some(Ok(msg)) => {
                                        let _ = self.receive_tx.send(msg).await;
                                    }
                                    Some(Err(e)) => {
                                        warn!("Error receiving from pipe: {e}");
                                        break;
                                    }
                                    None => {
                                        info!("Pipe server closed connection");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let jitter_millis = rand::thread_rng().gen_range(0..=100);
                    let sleep_dur = backoff + Duration::from_millis(jitter_millis);
                    debug!("Named pipe connection failed: {e}. Retrying in {sleep_dur:?}...");
                    tokio::time::sleep(sleep_dur).await;
                    backoff = (backoff * 3 / 2).min(max_backoff);
                }
            }
        }
    }

    #[cfg(windows)]
    async fn connect_named_pipe(pipe_name: &str) -> Result<NamedPipeClient, IpcError> {
        ClientOptions::new()
            .open(pipe_name)
            .map_err(|e| IpcError::ConnectionFailed(format!("Failed to open {pipe_name}: {e}")))
    }

    #[cfg(not(windows))]
    async fn connect_named_pipe(_pipe_name: &str) -> Result<(), IpcError> {
        Err(IpcError::UnsupportedPlatform)
    }
}
