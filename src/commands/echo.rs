use imperium_core::async_trait;
use imperium_core::{
    abstracts::commands::{
        Argument, ArgumentComponent, ArgumentDefinition, ArgumentFlags, ArgumentType,
        CommandContext, CommandMetadata, ImperiumCommand,
    },
    acl::{check_if_allowed, models::PermissionResponse},
    arg,
    dispatch::response::{CoreResponse, ResponseTone, SendError},
    dp,
};

use crate::reply;

pub struct EchoCommand;

#[async_trait]
impl ImperiumCommand for EchoCommand {
    async fn execute(&self, ctx: CommandContext<'_, '_, '_>) {
        let joined_args = ctx.args.arbitrary_positionals.join(" ");
        if joined_args.trim().is_empty() {
            reply!(
                ctx,
                "what are you asking me to repeat",
                ResponseTone::Confused
            );
            return;
        }
        let bypass_escape_arg: Option<bool> = arg!(ctx, "noescape");
        let bypass_escape = if bypass_escape_arg.unwrap_or(false) {
            match check_if_allowed(
                &Some(dp!["test", "echo_unescaped"]),
                None,
                &ctx.acl_resolved_permissions,
                None,
            ) {
                PermissionResponse::Allowed(_) => true,
                PermissionResponse::Denied(_) => {
                    reply!(
                        ctx,
                        "You do not have permission to do this.",
                        ResponseTone::Error
                    );
                    return;
                }
            }
        } else {
            false
        };
        match ctx
            .platform
            .send_response(&CoreResponse::new(ResponseTone::Ok).text(joined_args, !bypass_escape))
            .await
        {
            Ok(_) => {}
            Err(SendError::TooLong { .. }) => reply!(
                ctx,
                "Could not reply, message was too long for the platform..",
                ResponseTone::Error
            ),
            Err(e) => {
                eprintln!("Echo reply got an unexpected error: {e:?}")
            }
        }
    }
}

pub fn get_meta() -> CommandMetadata {
    CommandMetadata {
        name: "echo".to_string(),
        description: Some("Echoes positional arguments given to it.".to_string()),
        acl_path: /*Some(dp!["fun", "echo"])*/ None,
        minimum_anchor: None,
        argflags: ArgumentFlags {
            takes_arbitrary_positionals: true,
            takes_arbitrary_keywords: false,
        },
        arguments: vec![ArgumentDefinition {
            name: "noescape".into(),
            description: Some(
                "If set, bypasses escaping, but needs the test.echo_unescaped permission.".into(),
            ),
            acceptable_type: ArgumentType::Bool,
            component: ArgumentComponent::OptionalKeyword {
                default: Box::new(Argument::Null(())),
            },
        }],
    }
}
