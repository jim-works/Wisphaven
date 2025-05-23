use bevy::prelude::*;
use engine::controllers::Action;
use interfaces::scheduling::GameState;
use leafwing_input_manager::prelude::ActionState;
use quests::{ActiveQuest, CompletedQuest, Quest, QuestAsset};
use ui_core::{ButtonColors, get_large_text_style, get_text_style};
use ui_state::{UIScreen, UIState};

pub struct UiQuestPlugin;

impl Plugin for UiQuestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, toggle_quests)
            .add_systems(OnEnter(GameState::Game), spawn_ui)
            .add_systems(OnEnter(UIScreen::Quest), (update_quest_list, show))
            .add_systems(OnExit(UIScreen::Quest), hide);
    }
}

#[derive(Component)]
struct QuestUI;

#[derive(Component)]
struct QuestList;

#[derive(Component)]
struct QuestInfoPanel;

#[derive(Component)]
struct QuestTitle;

#[derive(Component)]
struct QuestDescription;

#[derive(Component, Clone)]
struct QuestRow {
    quest: Handle<QuestAsset>,
}

fn toggle_quests(
    mut next_state: ResMut<NextState<UIState>>,
    state: Res<State<UIState>>,
    action: Res<ActionState<Action>>,
) {
    if action.just_pressed(&Action::ToggleQuests) {
        info!("toggled");
        next_state.set(match state.get() {
            UIState::Default => UIState::Quest,
            UIState::Quest => UIState::Default,
            other_state @ _ => other_state.clone(),
        });
    }
}

fn spawn_ui(
    mut commands: Commands,
    ui_query: Query<(), With<QuestUI>>,
    asset_server: Res<AssetServer>,
) {
    if !ui_query.is_empty() {
        return;
    }
    let text_style = get_text_style(&asset_server);
    let large_text_style = get_large_text_style(&asset_server);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            Visibility::Hidden,
            BackgroundColor(Color::hsla(0., 0., 0.3, 0.8)),
            QuestUI,
            StateScoped(GameState::Game),
        ))
        .with_children(|panels| {
            panels.spawn((
                Node {
                    min_width: Val::Px(240.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::all(Val::Px(5.)),
                    ..default()
                },
                QuestList,
            ));
            panels
                .spawn((
                    Node {
                        width: Val::Auto,
                        height: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::scroll_y(),
                        padding: UiRect::all(Val::Px(5.)),
                        ..default()
                    },
                    QuestInfoPanel,
                ))
                .with_children(|info| {
                    info.spawn((
                        Node {
                            width: Val::Percent(100.),
                            ..default()
                        },
                        large_text_style.clone(),
                        Text::default(),
                        TextLayout::new(JustifyText::Left, LineBreak::WordOrCharacter),
                        QuestTitle,
                    ));
                    info.spawn((
                        Node {
                            width: Val::Percent(100.),
                            ..default()
                        },
                        text_style.clone(),
                        Text::default(),
                        TextLayout::new(JustifyText::Left, LineBreak::WordOrCharacter),
                        QuestDescription,
                    ));
                });
        });
}

fn update_quest_list(
    active_quests: Query<&Quest, With<ActiveQuest>>,
    completed_quests: Query<&Quest, With<CompletedQuest>>,
    parent_query: Query<Entity, With<QuestList>>,
    button_query: Query<Entity, With<QuestRow>>,
    quest_assets: Res<Assets<QuestAsset>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    // todo - optimize this if it becomes an issue
    let Ok(parent) = parent_query.get_single() else {
        error!("missing quest list entity when populating quests!");
        return;
    };
    //clear all quest buttons
    for button in button_query.iter() {
        if let Some(ec) = commands.get_entity(button) {
            ec.despawn_recursive();
        }
    }
    let button = (
        ButtonColors::default(),
        Node {
            width: Val::Percent(100.),
            border: UiRect::all(Val::Px(2.0)),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            margin: UiRect::all(Val::Px(4.)),
            ..default()
        },
        BorderColor(ButtonColors::default().default_border),
        BackgroundColor(ButtonColors::default().default_background),
        PickingBehavior {
            // want to be able to scroll the background
            should_block_lower: false,
            is_hoverable: true,
        },
        Button,
    );
    let text_style = get_text_style(&asset_server);
    let Some(mut parent_commands) = commands.get_entity(parent) else {
        error!("couldn't get quest list commands somehow");
        return;
    };
    parent_commands.despawn_descendants();
    parent_commands.with_children(|children| {
        if active_quests.is_empty() && completed_quests.is_empty() {
            children.spawn((
                Text::new("Your quests will show up here as you discover them."),
                TextLayout::new(JustifyText::Left, LineBreak::WordOrCharacter),
                text_style.clone(),
            ));
        }
        // spawn all active quests first
        if !active_quests.is_empty() {
            children.spawn((
                Text::new("Active Quests"),
                TextLayout::new(JustifyText::Left, LineBreak::WordOrCharacter),
                text_style.clone(),
            ));
        }
        for quest in active_quests.iter() {
            let Some(quest_asset) = quest_assets.get(&quest.0) else {
                error!("missing quest asset somehow");
                continue;
            };

            children
                .spawn((
                    button.clone(),
                    QuestRow {
                        quest: quest.0.clone(),
                    },
                ))
                .observe(on_quest_selected)
                .with_child((
                    Text::new(quest_asset.title.to_string()),
                    TextLayout::new(JustifyText::Center, LineBreak::WordOrCharacter),
                    text_style.clone(),
                ));
        }
        // completed quest section
        if !completed_quests.is_empty() {
            children.spawn((
                Text::new("Completed Quests"),
                TextLayout::new(JustifyText::Left, LineBreak::WordOrCharacter),
                text_style.clone(),
            ));
        }
        for quest in completed_quests.iter() {
            let Some(quest_asset) = quest_assets.get(&quest.0) else {
                error!("missing quest asset somehow");
                continue;
            };

            children
                .spawn((
                    button.clone(),
                    QuestRow {
                        quest: quest.0.clone(),
                    },
                ))
                .observe(on_quest_selected)
                .with_child((
                    Text::new(quest_asset.title.to_string()),
                    TextLayout::new(JustifyText::Center, LineBreak::WordOrCharacter),
                    text_style.clone(),
                ));
        }
    });
}

fn on_quest_selected(
    click: Trigger<Pointer<Down>>,
    button_query: Query<&QuestRow>,
    mut title_query: Query<&mut Text, (With<QuestTitle>, Without<QuestDescription>)>,
    mut desc_query: Query<&mut Text, (With<QuestDescription>, Without<QuestTitle>)>,
    quests: Res<Assets<QuestAsset>>,
) {
    info!("Quest selected {:?}", click.entity());
    let Ok(row) = button_query.get(click.entity()) else {
        error!("quest_selected on a button which doesn't have a quest row!!!! NOOOOIO!!!!!!");
        return;
    };
    let Some(quest) = quests.get(&row.quest) else {
        error!("invalid quest on quest button OH GOD WHY!");
        return;
    };
    for mut title in title_query.iter_mut() {
        title.0 = quest.title.to_string();
    }
    for mut desc in desc_query.iter_mut() {
        desc.0 = quest.description.to_string();
    }
}

fn show(mut query: Query<&mut Visibility, With<QuestUI>>) {
    for mut vis in query.iter_mut() {
        *vis.as_mut() = Visibility::Inherited;
    }
}

fn hide(mut query: Query<&mut Visibility, With<QuestUI>>) {
    for mut vis in query.iter_mut() {
        *vis.as_mut() = Visibility::Hidden;
    }
}
