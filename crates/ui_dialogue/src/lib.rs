#![feature(let_chains)]
use std::{sync::Arc, time::Duration};

use bevy::{
    audio::{PlaybackMode, Volume},
    prelude::*,
};
use dialogue::{ActiveDialogue, AdvanceDialogue};
use engine::controllers::Action;
use interfaces::scheduling::GameState;
use leafwing_input_manager::prelude::ActionState;
use rand::RngCore;
use ui_core::{ButtonColors, get_text_style};
use ui_state::UIState;

pub struct UiDialoguePlugin;

impl Plugin for UiDialoguePlugin {
    fn build(&self, app: &mut App) {
        // all dialogue processing happens in Tick
        app.init_resource::<DialogueUIState>()
            .add_systems(Startup, init)
            .add_systems(
                Update,
                (
                    update_display,
                    (advance_dialogue, progress_dialogue_text).run_if(in_state(UIState::Dialogue)),
                )
                    .chain(),
            )
            .add_systems(OnEnter(UIState::Dialogue), show_dialogue)
            .add_systems(OnExit(UIState::Dialogue), hide_dialogue);
    }
}

#[derive(Component)]
struct DialogueUI;
#[derive(Component)]
struct DialogueTextBox;
#[derive(Component)]
struct DialogueButtonContainer;

#[derive(Resource, Default)]
struct DialogueUIState {
    active_entity: Option<Entity>,
    ui: Option<UIType>,
    display_progress: f32,
    completed: Option<Duration>,
}

impl DialogueUIState {
    fn update_ui(&mut self, ui: Option<UIType>) {
        self.ui = ui;
        self.display_progress = 0.;
        self.completed = None;
    }
}

enum UIType {
    Message(Arc<str>),
    Buttons(Vec<Arc<str>>),
}

#[derive(Resource)]
struct DialogueUIResources {
    sounds: Vec<Handle<AudioSource>>,
}

fn update_display(
    changed_dialogue_query: Query<&ActiveDialogue, Changed<ActiveDialogue>>,
    all_dialogues: Query<Entity, With<ActiveDialogue>>,
    mut text_query: Query<
        (&mut Text, &mut Visibility),
        (With<DialogueTextBox>, Without<DialogueButtonContainer>),
    >,
    mut button_container_query: Query<
        (Entity, &mut Visibility),
        (With<DialogueButtonContainer>, Without<DialogueTextBox>),
    >,
    mut advance_writer: EventWriter<AdvanceDialogue>,
    mut state: ResMut<DialogueUIState>,
    mut next_ui_state: ResMut<NextState<UIState>>,
    curr_ui_state: Res<State<UIState>>,
    mut commands: Commands,
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
            match node.as_ref() {
                dialogue::DialogueNode::Message { message, .. } => {
                    state.update_ui(Some(UIType::Message(message.clone())));

                    if let Ok((mut text, mut vis)) = text_query.get_single_mut() {
                        text.0 = String::new();
                        *vis = Visibility::Inherited;
                    }
                    if let Ok((entity, mut vis)) = button_container_query.get_single_mut() {
                        *vis = Visibility::Hidden;
                        if let Some(mut ec) = commands.get_entity(entity) {
                            ec.despawn_descendants();
                        }
                    }
                }
                dialogue::DialogueNode::Response { options, .. } => {
                    state.update_ui(Some(UIType::Buttons(
                        options.iter().map(|opt| opt.message.clone()).collect(),
                    )));

                    if let Ok((mut text, mut vis)) = text_query.get_single_mut() {
                        text.0 = String::new();
                        *vis = Visibility::Hidden;
                    }
                    if let Ok((_, mut vis)) = button_container_query.get_single_mut() {
                        *vis = Visibility::Inherited;
                        commands.run_system_cached(spawn_buttons);
                    }
                }
                dialogue::DialogueNode::Decision { .. } | dialogue::DialogueNode::Jump { .. } => {
                    error!(
                        "trying to display non-visual node. this shouldn't happen ever. advancing dialogue...{:?}",
                        dialogue
                    );
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
        }
        None => {
            // dialogue ended, wait for user to advance
            return;
        }
    }
}

fn advance_dialogue(
    mut state: ResMut<DialogueUIState>,
    mut advance_writer: EventWriter<AdvanceDialogue>,
    input: Res<ActionState<Action>>,
    time: Res<Time>,
) {
    if !input.just_pressed(&Action::SkipDialogue) || !matches!(state.ui, Some(UIType::Message(_))) {
        return;
    }
    let Some(dialogue_entity) = state.active_entity else {
        return;
    };
    // i accidentally skip dialogue in games sometimes, so maybe this will help? idk
    let dialogue_skip_cooldown_time = Duration::from_millis(100);
    match state.completed {
        Some(finish_time) => {
            if finish_time + dialogue_skip_cooldown_time <= time.elapsed() {
                advance_writer.send(AdvanceDialogue {
                    dialogue_entity,
                    selected_response: None,
                });
            }
        }
        None => {
            state.display_progress = f32::INFINITY;
            state.completed = Some(time.elapsed())
        }
    }
}

fn progress_dialogue_text(
    mut state: ResMut<DialogueUIState>,
    resources: Res<DialogueUIResources>,
    time: Res<Time>,
    mut text_query: Query<&mut Text, With<DialogueTextBox>>,
    mut last_progress: Local<usize>,
    mut commands: Commands,
) {
    if let Some(UIType::Message(target_message)) = &state.ui {
        if target_message.len() == 0 {
            //very important cause i do some -1's in here
            return;
        }
        if state.completed.is_some() {
            if *last_progress <= target_message.len() - 1 {
                for mut text in text_query.iter_mut() {
                    text.0 = target_message.to_string();
                    *last_progress = target_message.len() - 1;
                }
            }
            return;
        }
        let mut rng = rand::thread_rng();
        let speed = 40.0; //characters per second
        let len = ((state.display_progress * speed) as usize).min(target_message.len() - 1);
        if len != *last_progress {
            // this api causes a lot of allocations T_T
            let message = target_message[0..len].to_string();
            if let Some(sound) = util::get_wrapping(&resources.sounds, rng.next_u32() as usize) {
                commands.spawn((
                    AudioPlayer(sound.clone()),
                    PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        volume: Volume::new(0.25),
                        speed: 0.5,
                        ..default()
                    },
                ));
            }
            for mut text in text_query.iter_mut() {
                text.0 = message.clone();
            }
        }
        if len >= target_message.len() - 1 {
            state.completed = Some(time.elapsed());
        }
        *last_progress = len;
    }

    state.display_progress += time.delta_secs();
}

fn show_dialogue(mut inventory_query: Query<&mut Visibility, With<DialogueUI>>) {
    for mut vis in inventory_query.iter_mut() {
        info!("showing dialogue");
        *vis.as_mut() = Visibility::Inherited;
    }
}

fn hide_dialogue(mut query: Query<&mut Visibility, With<DialogueUI>>) {
    for mut vis in query.iter_mut() {
        info!("hiding dialogue");
        *vis.as_mut() = Visibility::Hidden;
    }
}

fn spawn_buttons(
    mut commands: Commands,
    button_container_query: Query<Entity, With<DialogueButtonContainer>>,
    state: Res<DialogueUIState>,
    asset_server: Res<AssetServer>,
) {
    let Some(dialogue_entity) = state.active_entity else {
        error!("can't find da active dialogue entity to spawn da dialogue buttons :'(");
        return;
    };
    let Ok(container_entity) = button_container_query.get_single() else {
        error!("can't find da container to spawn da dialogue buttons WTFFFFF!!!!");
        return;
    };
    let Some(UIType::Buttons(button_texts)) = &state.ui else {
        error!("can't find da texts to spawn da dialogue buttons! FRICK!");
        return;
    };
    let Some(mut container_ec) = commands.get_entity(container_entity) else {
        error!("can't find da commands to spawn da dialogue buttons WTFFFFF!!!!");
        return;
    };
    container_ec.with_children(|children| {
        for (i, btext) in button_texts.iter().enumerate() {
            children
                .spawn((
                    Node {
                        height: Val::Px(48.),
                        width: Val::Percent(100.),
                        border: UiRect::all(Val::Px(2.)),
                        margin: UiRect::all(Val::Px(1.)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexStart,
                        justify_content: JustifyContent::FlexStart,
                        ..default()
                    },
                    Button,
                    ButtonColors::default(),
                    BorderColor(ButtonColors::default().default_border),
                    BackgroundColor(ButtonColors::default().default_background),
                ))
                .observe(
                    move |mut click: Trigger<Pointer<Click>>,
                          mut writer: EventWriter<AdvanceDialogue>| {
                        writer.send(AdvanceDialogue {
                            dialogue_entity,
                            selected_response: Some(i),
                        });
                        click.propagate(false);
                    },
                )
                .with_child((
                    Node {
                        margin: UiRect::horizontal(Val::Px(5.)),
                        ..default()
                    },
                    Text(btext.to_string()),
                    get_text_style(&asset_server).clone(),
                ));
        }
    });
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
            Name::new("dialogue ui"),
            PickingBehavior::IGNORE,
            Visibility::Hidden,
        ))
        .with_children(|background| {
            background
                // background
                .spawn((
                    Node {
                        height: Val::Auto,
                        width: Val::Percent(100.),
                        border: UiRect::all(Val::Px(5.)),
                        padding: UiRect::left(Val::Px(5.)),
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
                        TextLayout::new(JustifyText::Left, LineBreak::WordOrCharacter),
                        Name::new("dialogue text"),
                        get_text_style(&asset_server),
                        DialogueTextBox,
                    ));
                    // buttons
                    items.spawn((
                        Node {
                            width: Val::Percent(100.),
                            height: Val::Percent(100.),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            ..default()
                        },
                        Name::new("dialogue buttons"),
                        DialogueButtonContainer,
                    ));
                });
        });

    commands.insert_resource(DialogueUIResources {
        sounds: vec![
            asset_server.load("sounds/interface/click.ogg"),
            asset_server.load("sounds/interface/click1.ogg"),
            asset_server.load("sounds/interface/click2.ogg"),
            asset_server.load("sounds/interface/click3.ogg"),
        ],
    });
}
