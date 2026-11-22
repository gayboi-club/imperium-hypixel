use imperium_core::{
    abstracts::commands::PlatformContext,
    async_trait,
    dispatch::response::{CoreResponse, LengthCountBasedOn, SendError as ReplySendError},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    render::RenderLogic,
    socket_handler::{DataFromRelay, DataToRelay},
};

pub struct HypixelPlatform {
    pub original_msg: DataFromRelay,
    pub send_handle: UnboundedSender<DataToRelay>,
}

#[macro_export]
macro_rules! reply {

    ($ctx:expr, $text:expr, $tone:expr) => {{
        let cr = imperium_core::dispatch::response::CoreResponse::new($tone)
            .text($text, false);

        $crate::reply!(cr $ctx, cr);
    }};

    (cr $ctx:expr, $cr:expr) => {{
        match $ctx.platform.send_response(&$cr).await {
            Ok(_) => {}
            Err(imperium_core::dispatch::response::SendError::TooLong{max_length, sent_length, based_on}) => {
                eprintln!("Too long case hit: max: {max_length}, sent: {sent_length} {based_on:?}");
                let __reply_err = imperium_core::dispatch::response::CoreResponse::new(imperium_core::dispatch::response::ResponseTone::Error)
                .text("There is a bug, please contact support. (Message too long case)".to_string(), false);
                if let Err(e) = $ctx.platform.send_response(&__reply_err).await {
                    eprintln!("what the fuck, reply! failed to send the too long msg, rate limits/slowmode? {e}")
                }
            }
            Err(e) => {
                eprintln!("... fuck: {e:?}")
            }
        }
    }};
}

#[async_trait]
impl PlatformContext for HypixelPlatform {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_platform_id(&self) -> Option<&'static str> {
        Some("hypixel")
    }

    async fn send_response(&self, resp: &CoreResponse) -> Result<(), ReplySendError> {
        tracing::debug!("{resp:?}");

        let renderer = RenderLogic;
        let rendered = renderer.render(&resp);

        if rendered.chars().count() >= 256 {
            return Err(ReplySendError::TooLong {
                max_length: 256,
                sent_length: rendered.chars().count(),
                based_on: LengthCountBasedOn::Characters,
            });
        }

        let _ = self.send_handle.send(DataToRelay {
            message: rendered,
            addressing_ign: self.original_msg.ign.clone(),
            addressing_uuid: self.original_msg.uuid.clone(),
            ephemeral: false,
        });

        Ok(())
    }
}
