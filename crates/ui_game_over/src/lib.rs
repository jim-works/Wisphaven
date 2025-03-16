use bevy::prelude::*;
use interfaces::scheduling::GameState;
use world::atmosphere::Calendar;

use ui_core::{get_large_text_style, get_text_style, ButtonColors, TRANSLUCENT_PANEL_BACKGROUND};

pub struct UIGameOverPlugin;

impl Plugin for UIGameOverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::GameOver), create_game_over_ui);
    }
}

#[derive(Component)]
struct GameOverUI;

fn create_game_over_ui(
    mut commands: Commands,
    calendar: Res<Calendar>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn((
            StateScoped(GameState::GameOver),
            GameOverUI,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            PickingBehavior::IGNORE,
            BackgroundColor(TRANSLUCENT_PANEL_BACKGROUND),
        ))
        .with_children(|sections| {
            sections
                // game over title
                .spawn((
                    Node {
                        height: Val::Percent(25.),
                        width: Val::Percent(100.),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Text::new("Game Over!"),
                    TextLayout::new_with_justify(JustifyText::Center),
                    get_large_text_style(&asset_server),
                    BackgroundColor(TRANSLUCENT_PANEL_BACKGROUND),
                ));
            // time survived label
            sections.spawn((
                Node {
                    height: Val::Percent(25.),
                    width: Val::Percent(100.),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new(format!("Time of death: {}.", calendar.time)),
                TextLayout::new_with_justify(JustifyText::Center),
                get_text_style(&asset_server),
            ));
            // return to menu button
            sections
                .spawn((
                    ButtonColors::default(),
                    Node {
                        width: Val::Px(256.),
                        height: Val::Px(48.0),
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
                    Button,
                ))
                .observe(
                    |_trigger: Trigger<Pointer<Click>>,
                     mut next_state: ResMut<NextState<GameState>>| {
                        next_state.set(GameState::Menu);
                    },
                )
                .with_children(|text| {
                    text.spawn((Text::new("Main Menu"), get_text_style(&asset_server)));
                });
        });
}
