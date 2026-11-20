use imperium_core::{abstracts::commands::CommandTreeNode, dispatch::Dispatcher};

mod coinflip;

pub fn register_commands(dispatcher: &mut Dispatcher) {
    dispatcher.set_root_command_tree_node(CommandTreeNode::Command(
        coinflip::get_meta(),
        Box::new(coinflip::CoinflipCommand),
    ));
}
