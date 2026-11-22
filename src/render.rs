use imperium_core::dispatch::response::{
    Component, CoreResponse, ResponseTone, SeparatorType, default_kv_list_renderer,
    default_list_renderer, default_ts_renderer,
};
use std::fmt::Write;
pub struct RenderLogic;

impl RenderLogic {
    fn escape<'a>(input: &'a str) -> String {
        if input.is_empty() {
            return "...?".into();
        }

        if input.chars().next().expect("check the last if statement") == '/' {
            return format!(".{input}").into();
        }

        input.into()
    }

    pub fn render(&self, cr: &CoreResponse) -> String {
        let mut buf = String::new();

        let tone_prefix = match cr.view().0 {
            ResponseTone::Ok => None,
            ResponseTone::Warning => Some("⚠"),
            ResponseTone::Error => Some("!"),
            ResponseTone::Confused => Some("?"),
        };

        if let Some(v) = tone_prefix {
            buf.push('(');
            buf.push_str(v);
            buf.push_str(") ");
        }

        for c in cr.view().1 {
            self.render_component(&mut buf, c, 0);
        }

        buf
    }

    fn render_component(&self, buf: &mut String, c: &Component, idx: usize) {
        macro_rules! recursion_go_brr {
            () => {
                |buf, c, idx| self.render_component(buf, c, idx + 1)
            };
        }
        if idx > 500 && option_env!("IMPERIUM_SHUT_UP_DEFAULT_RENDERER_RECURSION_WARNING").is_none()
        {
            eprintln!(
                "idx is absurdly high on render_component: {}, gl (silence this warning by setting IMPERIUM_SHUT_UP_DEFAULT_RENDERER_RECURSION_WARNING at compile time)",
                idx
            ); // we don't need to panic here, stack overflow is a thing
        }
        match c {
            Component::Custom { v } => {
                match v
                    .0
                    .render(buf, &|buf, c, idx| self.render_component(buf, c, idx + 1))
                {
                    None => (),
                    Some(v) => {
                        if cfg!(debug_assertions) {
                            panic!(
                                "We got an unexpected Some(v): {v:#?}\nNote: This is the default renderer, if you need to pass around metadata, write your own."
                            );
                        }
                        eprintln!(
                            "Defalt renderer got custom render metadata, this is a bug: {v:#?}"
                        );
                    }
                }
            }
            Component::Text {
                value,
                needs_escaping,
            } => write!(
                buf,
                "{}",
                match needs_escaping {
                    false => value.to_string(),
                    true => RenderLogic::escape(&value),
                }
            )
            .expect("write cannot fail for String.. can it?"),
            Component::User { user: u, .. } => write!(
                buf,
                "{}",
                u.name.clone().unwrap_or("no username? tf?".into())
            )
            .unwrap(),
            Component::Role { role: r, .. } => write!(
                buf,
                "[{}]",
                r.name.clone().unwrap_or("no role name, weird".into()),
            )
            .unwrap(),
            Component::Timestamp {
                time,
                display_as_relative,
            } => default_ts_renderer(buf, time, *display_as_relative),
            Component::List { value, style } => {
                default_list_renderer(buf, value, *style, idx, recursion_go_brr!())
            }
            Component::KeyValueList { value } => {
                default_kv_list_renderer(buf, value, idx, recursion_go_brr!())
            }
            Component::Separator(typ) => buf.push_str(match typ {
                SeparatorType::Soft => " | ",
                SeparatorType::Paragraph => " -- ",
                SeparatorType::Hard => " ---- ",
            }),
        }
    }
}
