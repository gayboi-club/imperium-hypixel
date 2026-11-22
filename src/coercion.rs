use imperium_core::{
    abstracts::{
        commands::{Argument, ArgumentType},
        general::{CoreUser, Namespace},
    },
    astr,
    dispatch::textparsing::arguments::parsing::ArgumentCoercionLogic,
};

pub struct CoercionLogic;

fn parse_uid_from_ping(input: &str) -> Option<&str> {
    None
    // if input.chars().all(|c| c.is_ascii_digit()) {
    //     Some(input)
    // } else if input.starts_with("<@") && input.ends_with(">") && input.len() > 3 {
    //     Some(&input[2..input.len() - 1])
    // } else {
    //     None
    // }
}

fn parse_bool(input: &str) -> Option<bool> {
    Some(match input.to_ascii_lowercase().as_str() {
        "true" => true,
        "false" => false,
        "1" => true,
        "0" => false,

        _ => return None,
    })
}

impl ArgumentCoercionLogic for CoercionLogic {
    fn coerce(input: &str, ns: &Namespace, target: ArgumentType) -> Argument {
        match target {
            ArgumentType::User => {
                let attempted_parse_user = parse_uid_from_ping(input);
                if let Some(parsed_user) = attempted_parse_user {
                    Argument::User(CoreUser {
                        id: parsed_user.into(),
                        name: None,
                        ns: ns.clone(),
                    })
                } else {
                    return astr!(input); // thank you ! type
                }
            }

            ArgumentType::Bool => {
                let attempted_parse = parse_bool(input);
                if let Some(v) = attempted_parse {
                    Argument::Bool(v)
                } else {
                    return astr!(input);
                }
            }

            _ => astr!(input),
        }
    }
}
