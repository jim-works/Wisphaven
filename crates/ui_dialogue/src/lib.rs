#![feature(let_chains)]
use bevy::prelude::*;
use dialogue::{ActiveDialogue, AdvanceDialogue};
use interfaces::scheduling::GameState;
use ui_core::get_text_style;
use ui_state::UIState;

pub struct UiDialoguePlugin;

impl Plugin for UiDialoguePlugin {
    fn build(&self, app: &mut App) {
        // all dialogue processing happens in Tick
        app.init_resource::<DialogueUIState>()
            .add_systems(Startup, init)
            .add_systems(Update, update_display)
            .add_systems(OnEnter(UIState::Dialogue), show_dialogue)
            .add_systems(OnExit(UIState::Dialogue), hide_dialogue);
    }
}

#[derive(Component)]
struct DialogueUI;
#[derive(Component)]
struct DialogueTextBox;

#[derive(Resource, Default)]
struct DialogueUIState {
    active_entity: Option<Entity>,
    display_progress: f32,
}

fn update_display(
    changed_dialogue_query: Query<&ActiveDialogue, Changed<ActiveDialogue>>,
    all_dialogues: Query<Entity, With<ActiveDialogue>>,
    mut text_query: Query<&mut Text, With<DialogueTextBox>>,
    mut advance_writer: EventWriter<AdvanceDialogue>,
    mut state: ResMut<DialogueUIState>,
    mut next_ui_state: ResMut<NextState<UIState>>,
    curr_ui_state: Res<State<UIState>>,
) {
    if let Some(curr_entity) = &state.active_entity
        && !all_dialogues.contains(*curr_entity)
    {
        *state = default();
        state.active_entity = all_dialogues.iter().next();
    } else if state.active_entity.is_none() {
        *state = default();
        state.active_entity = all_dialogues.iter().next();
    }

    let Some(dialogue_entity) = state.active_entity.clone() else {
        // no active entity
        if matches!(curr_ui_state.get(), UIState::Dialogue) {
            info!("no active dialog entity");
            next_ui_state.set(UIState::Default);
        }
        return;
    };
    let Ok(dialogue) = changed_dialogue_query.get(dialogue_entity) else {
        if !all_dialogues.contains(dialogue_entity)
            && matches!(curr_ui_state.get(), UIState::Dialogue)
        {
            info!("active entity has no dialogue");
            next_ui_state.set(UIState::Default);
        }
        return;
    };
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
                dialogue::DialogueNode::Decision { .. } | dialogue::DialogueNode::Jump { .. } => {
                    error!("trying to display non-visual node. advancing dialogue...");
                    advance_writer.send(AdvanceDialogue {
                        dialogue_entity,
                        selected_response: None,
                    });
                    return;
                }
            };
            if !matches!(curr_ui_state.get(), UIState::Dialogue) {
                // we are now in an active dialogue
                info!("entering dialogue");
                next_ui_state.set(UIState::Dialogue);
            }
            if let Ok(mut text) = text_query.get_single_mut() {
                text.0 = String::from(message);
            }
        }
        None => {
            // dialogue ended, wait for user to advance
            return;
        }
    }
}

fn show_dialogue(mut inventory_query: Query<&mut Visibility, With<DialogueUI>>) {
    for mut vis in inventory_query.iter_mut() {
        info!("showing dialogue");
        *vis.as_mut() = Visibility::Inherited;
    }
}

fn hide_dialogue(mut query: Query<&mut Visibility, (With<DialogueUI>)>) {
    for mut vis in query.iter_mut() {
        info!("hiding dialogue");
        *vis.as_mut() = Visibility::Hidden;
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    // todo - make speech bubble customizable
    let background_color = Color::srgb(199. / 255., 207. / 255., 204. / 255.);
    let border_color = Color::srgb(87. / 255., 114. / 255., 119. / 255.);
    commands
        .spawn((
            StateScoped(GameState::Game),
            DialogueUI,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            PickingBehavior::IGNORE,
            Visibility::Hidden,
        ))
        .with_children(|background| {
            background
                // background
                .spawn((
                    Node {
                        height: Val::Percent(50.),
                        width: Val::Percent(100.),
                        border: UiRect::all(Val::Px(5.)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexStart,
                        justify_content: JustifyContent::FlexStart,
                        ..default()
                    },
                    BackgroundColor(background_color),
                    BorderColor(border_color),
                ))
                .with_children(|items| {
                    //text
                    items.spawn((
                        Node {
                            width: Val::Percent(100.),
                            height: Val::Percent(100.),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexEnd,
                            ..default()
                        },
                        Text::new("Default text"),
                        TextLayout::new_with_justify(JustifyText::Center),
                        get_text_style(&asset_server),
                        DialogueTextBox,
                    ));
                });
        });
}
