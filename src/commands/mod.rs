use imperium_core::{abstracts::commands::CommandTreeNode, dispatch::Dispatcher};

mod coinflip;
mod echo;

pub fn register_commands(dispatcher: &mut Dispatcher) {
    dispatcher.set_root_command_tree_node(CommandTreeNode::Command(
        coinflip::get_meta(),
        Box::new(coinflip::CoinflipCommand),
    ));
    dispatcher.set_root_command_tree_node(CommandTreeNode::Command(
        echo::get_meta(),
        Box::new(echo::EchoCommand),
    ));
}
