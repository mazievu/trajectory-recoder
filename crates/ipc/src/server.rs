use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

use crate::codec::MsgPackCodec;
use crate::error::IpcError;
use crate::protocol::IpcMessage;
use crate::security::{DEFAULT_PIPE_SDDL, PipeSecurityAttributes};

pub struct IpcServer {
    pipe_name: String,
    sddl: String,
    incoming_tx: mpsc::Sender<IpcMessage>,
    cancel_token: CancellationToken,
}

impl IpcServer {
    pub fn new(
        pipe_name: impl Into<String>,
        incoming_tx: mpsc::Sender<IpcMessage>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            pipe_name: pipe_name.into(),
            sddl: DEFAULT_PIPE_SDDL.to_string(),
            incoming_tx,
            cancel_token,
        }
    }

    pub fn with_sddl(mut self, sddl: impl Into<String>) -> Self {
        self.sddl = sddl.into();
        self
    }

    pub async fn run(self) -> Result<(), IpcError> {
        info!("Starting Named Pipe IPC server on {}", self.pipe_name);
        let sec_attrs = PipeSecurityAttributes::from_sddl(&self.sddl)?;

        #[cfg(windows)]
        {
            let mut is_first_instance = true;

            loop {
                let mut server = match Self::create_pipe_instance(
                    &self.pipe_name,
                    is_first_instance,
                    &sec_attrs,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to create named pipe instance: {e}. Retrying in 1s...");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };
                is_first_instance = false;

                tokio::select! {
                    _ = self.cancel_token.cancelled() => {
                        info!("IPC Server received cancellation signal, stopping listener.");
                        break;
                    }
                    connect_res = server.connect() => {
                        if let Err(e) = connect_res {
                            warn!("Pipe connect error: {e}");
                            continue;
                        }

                        let client_tx = self.incoming_tx.clone();
                        let cancel = self.cancel_token.clone();

                        tokio::spawn(async move {
                            let framed = Framed::new(server, MsgPackCodec::<IpcMessage>::default());
                            let (_sink, mut stream) = framed.split();

                            loop {
                                tokio::select! {
                                    _ = cancel.cancelled() => break,
                                    msg = stream.next() => {
                                        match msg {
                                            Some(Ok(message)) => {
                                                if client_tx.send(message).await.is_err() {
                                                    break;
                                                }
                                            }
                                            Some(Err(e)) => {
                                                warn!("Client decoding error: {e}");
                                                break;
                                            }
                                            None => break, // Client disconnected
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = sec_attrs;
            Err(IpcError::UnsupportedPlatform)
        }
    }

    #[cfg(windows)]
    fn create_pipe_instance(
        pipe_name: &str,
        first_instance: bool,
        sec_attrs: &PipeSecurityAttributes,
    ) -> Result<NamedPipeServer, IpcError> {
        let mut opts = ServerOptions::new();
        opts.first_pipe_instance(first_instance);
        opts.max_instances(255);
        opts.in_buffer_size(65536);
        opts.out_buffer_size(65536);

        unsafe {
            opts.create_with_security_attributes_raw(
                pipe_name,
                sec_attrs.as_raw_ptr() as *mut std::ffi::c_void,
            )
            .map_err(|e| {
                IpcError::PipeCreationError(format!("CreateNamedPipe failed for {pipe_name}: {e}"))
            })
        }
    }
}
