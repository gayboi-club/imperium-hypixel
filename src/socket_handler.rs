use serde::Deserialize;
use serde_json::json;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::{
        broadcast,
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    },
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::IPC_SEPARATOR;

/// returns in this order:
///
/// a handle to queue messages to be sent to the relay
/// a broadcast handle to read messages we get
/// a join handle to the writer task
/// a join handle to the reader task
pub async fn create_socket_channels(
    socket: UnixStream,
    cancellation_token: CancellationToken,
) -> (
    UnboundedSender<DataToRelay>,
    broadcast::Receiver<DataFromRelay>,
    JoinHandle<()>,
    JoinHandle<()>,
) {
    let (reader, writer) = socket.into_split();

    let (write_sender, write_receiver) = unbounded_channel();

    let write_task_handle =
        handle_write_half(writer, cancellation_token.clone(), write_receiver).await;

    let (read_submit, read_reciever) = broadcast::channel::<DataFromRelay>(64);

    let read_task_handle = handle_read_half(reader, cancellation_token, read_submit);

    (
        write_sender,
        read_reciever,
        write_task_handle,
        read_task_handle,
    )
}

#[derive(Debug)]
pub struct DataToRelay {
    pub message: String,
    pub addressing_ign: String,
    pub addressing_uuid: String,
    pub ephemeral: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DataFromRelay {
    pub ign: String,
    pub uuid: String,
    pub role: Option<String>,
    pub message: String,
}

impl DataToRelay {
    pub fn serialize(&self) -> String {
        let mut str = json! ({
            "message": self.message,
            "addressing": {
                "ign": self.addressing_ign,
                "uuid": self.addressing_uuid
            },
            "ephemeral": self.ephemeral
        })
        .to_string();
        str.push(IPC_SEPARATOR as char);
        str
    }
}
#[instrument]
async fn handle_write_half(
    mut write_half: OwnedWriteHalf,
    cancellation_token: CancellationToken,
    mut recv: UnboundedReceiver<DataToRelay>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;

                _ = cancellation_token.cancelled() => {
                    tracing::info!("Cancellation token fired!");
                    break;
                }

                v = recv.recv() => {
                    match v {
                        Some(packet_to_send) => {
                            let serialized = packet_to_send.serialize();

                            tokio::select! {
                                biased;

                                _ = cancellation_token.cancelled() => {
                                    tracing::info!("Cancellation during write!");
                                    break;
                                }

                                result = write_half.write_all(serialized.as_bytes()) => {
                                    if let Err(e) = result {
                                        tracing::error!("write_all shat itself! cancelling... {e:?}");
                                        cancellation_token.cancel();
                                        break;
                                    }
                                }
                            }

                       }
                        None => {
                            tracing::info!("Data -> Relay channel was closed, write half shutting down.");
                            break;
                        }
                    }
                }

            }
        }
    })
}

#[instrument]
fn handle_read_half(
    read_half: OwnedReadHalf,
    cancellation_token: CancellationToken,
    broadcast: broadcast::Sender<DataFromRelay>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(read_half);

        loop {
            let mut buf = Vec::new();

            tokio::select! {
                biased;

                _ = cancellation_token.cancelled() => {
                    tracing::info!("Cancellation token fired!");
                    break;
                }

                v = reader.read_until(IPC_SEPARATOR, &mut buf) => {
                    let mut in_use_buf = Vec::new();
                    std::mem::swap(&mut in_use_buf, &mut buf);
                    if let Err(e) = v {
                        tracing::error!("Reading failed, shutting down! {e:?}");
                        cancellation_token.cancel();
                        break;
                    } else if let Ok(0) = v {
                        tracing::warn!("EOF on read, shutting down!");
                        cancellation_token.cancel();
                        break;
                    } else if in_use_buf.last().expect("We checked above that vec should not be empty.") != &IPC_SEPARATOR {
                        tracing::error!("Socket closed with an unfinished payload, shutting down..., {in_use_buf:?}");
                        cancellation_token.cancel();
                        break;
                    }

                    if let Ok(deserialized) = serde_json::from_slice::<DataFromRelay>(&in_use_buf){
                        if let Err(e) = broadcast.send(deserialized) {
                            tracing::error!("broadcast.send on DataFromRelay returned Err({e:?}), please explain to me why this branch fired. docs say that future calls shouldn't error so here we are");
                        }
                    } else {
                       tracing::error!("Relay gave us unparsable data, tf? closing due to assumptions being threatened. {:?}", String::from_utf8_lossy_owned(in_use_buf));

                       cancellation_token.cancel();
                       break;

                    }
                }
            }
        }
    })
}
