use bevy::prelude::*;
use dialogue::{ActiveDialogue, AdvanceDialogue};

pub struct UiDialoguePlugin;

impl Plugin for UiDialoguePlugin {
    fn build(&self, app: &mut App) {
        // all dialogue processing happens in Tick
        app.add_systems(Update, update_display);
    }
}

fn update_display(
    dialogue_query: Query<(Entity, &ActiveDialogue), Changed<ActiveDialogue>>,
    mut advance_writer: EventWriter<AdvanceDialogue>,
) {
    for (dialogue_entity, dialogue) in dialogue_query.iter() {
        match &dialogue.active_node {
            Some(node) => {
                let message = match node.as_ref() {
                    dialogue::DialogueNode::Message {
                        id,
                        message,
                        effects,
                        node,
                    } => message.as_str(),
                    dialogue::DialogueNode::Response { id, options } => "response node",
                    dialogue::DialogueNode::Decision { .. }
                    | dialogue::DialogueNode::Jump { .. } => {
                        error!("trying to display non-visual node. advancing dialogue...");
                        advance_writer.send(AdvanceDialogue {
                            dialogue_entity,
                            selected_response: None,
                        });
                        continue;
                    }
                };
                info!("active node with message: {}", message);
            }
            None => {
                info!("no active dialogue node :(");
            }
        }
    }
}
