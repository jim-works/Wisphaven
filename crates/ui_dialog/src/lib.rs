use bevy::prelude::*;
use dialog::ActiveDialog;

pub struct UiDialogPlugin;

impl Plugin for UiDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_display);
    }
}

fn update_display(dialog_query: Query<&ActiveDialog, Changed<ActiveDialog>>) {
    for dialog in dialog_query.iter() {
        match &dialog.active_node {
            Some(node) => {
                let message = match node.as_ref() {
                    dialog::DialogNode::Decision { id, choices } => "decision node",
                    dialog::DialogNode::Message {
                        id,
                        message,
                        effects,
                        node,
                    } => message.as_str(),
                    dialog::DialogNode::Response { id, options } => "response node",
                    dialog::DialogNode::Jump { jump_to, id } => "jump node",
                };
                info!("active node with message: {}", message);
            }
            None => {
                info!("no active dialog node :(");
            }
        }
    }
}
