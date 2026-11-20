use std::{thread::sleep, time::Duration};

use imperium_core::{
    abstracts::{
        commands::PlatformContext,
        general::{CoreUser, Namespace},
    },
    acl::models::ResolvedUserPermissions,
    dispatch::{
        Dispatcher, response::ResponseTone, textparsing::arguments::parsing::ArgumentCoercionLogic,
    },
};
use tokio::{net::UnixStream, sync::mpsc::UnboundedSender};
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument, warn};
use tracing_subscriber::{EnvFilter, fmt};

use commands::register_commands;

use crate::{
    coercion::CoercionLogic,
    ctx::HypixelPlatform,
    socket_handler::{DataFromRelay, DataToRelay, create_socket_channels},
};

pub const IPC_SEPARATOR: u8 = '\n' as u8;
const IPC_PATH: &str = "/tmp/imperium-hypixel.sock";

mod coercion;
mod commands;
mod ctx;
mod socket_handler;

#[instrument]
async fn wait_until_socket_exists() -> UnixStream {
    info!("attempting to connect...");
    match UnixStream::connect(IPC_PATH).await {
        Ok(v) => v,
        Err(e) => {
            warn!("Couldn't connect to socket, retrying in a sec... {e:?}");
            sleep(Duration::from_secs(1));
            Box::pin(wait_until_socket_exists()).await
        }
    }
}

struct DummyContext<'a> {
    platform: &'a dyn PlatformContext,
}

#[tokio::main(flavor = "current_thread")]
#[instrument]
async fn main() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    fmt().with_env_filter(filter).init();

    let socket = wait_until_socket_exists().await;

    info!("Socket connected!, {socket:?}");

    let cancellation_token = CancellationToken::new();

    let (submit_msg_send_queue, mut read_messages_from_hypixel, writer_handle, reader_handle) =
        create_socket_channels(socket, cancellation_token.clone()).await;

    let mut dispatcher = Dispatcher::new("hypixel".into());

    register_commands(&mut dispatcher);

    while let Ok(msg) = read_messages_from_hypixel.recv().await {
        let ns = Namespace {
            platform: "hypixel".into(),
            scope: "global".into(),
        };

        let pltctx = HypixelPlatform {
            original_msg: msg.clone(),
            send_handle: submit_msg_send_queue.clone(),
        };

        match dispatcher.dispatch_high_level::<CoercionLogic>(&msg.message, &pltctx, "global", "?")
        {
            Ok((meta, cmd, args)) => {
                match dispatcher
                    .validate_acl_and_maybe_run(
                        meta,
                        ResolvedUserPermissions::default(),
                        args,
                        ns.clone(),
                        cmd,
                        &CoreUser {
                            id: msg.uuid.clone().into(),
                            name: Some(msg.ign.clone().into()),
                            ns,
                        },
                        &pltctx,
                        None,
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(e) => reply!(
                        DummyContext { platform: &pltctx },
                        format!("Permission denied: {e}"),
                        ResponseTone::Error
                    ),
                }
            }
            Err(e) => todo!("error handling does not exist o.o {e:?}"),
        }
    }
}
