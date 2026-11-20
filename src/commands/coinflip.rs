use imperium_core::abstracts::commands::ArgumentFlags;
use imperium_core::async_trait;
use imperium_core::{
    abstracts::commands::{CommandContext, CommandMetadata, ImperiumCommand},
    dispatch::response::ResponseTone,
    dp,
};
use rand::random;

use crate::reply;

pub struct CoinflipCommand;

#[async_trait]
impl ImperiumCommand for CoinflipCommand {
    async fn execute(&self, ctx: CommandContext<'_, '_, '_>) {
        reply!(
            ctx,
            match random::<bool>() {
                true => "Heads :3",
                false => "Tails owo",
            },
            ResponseTone::Ok
        )
    }
}

pub fn get_meta() -> CommandMetadata {
    CommandMetadata {
        name: "cf".to_string(),
        description: Some("Flips a coin!".to_string()),
        // acl_path: Some(dp!["fun", "coinflip"]),
        acl_path: None,
        minimum_anchor: None,
        argflags: ArgumentFlags::default(),
        arguments: vec![],
    }
}
