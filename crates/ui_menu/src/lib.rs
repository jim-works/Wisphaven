use bevy::{app::AppExit, prelude::*};
use interfaces::{
    components::{Hand, HandState},
    scheduling::GameState,
};
use serialization::{LevelCreationInput, SavedLevels};
use std::time::Duration;

use bevy_simple_text_input::{TextInput, TextInputTextColor, TextInputTextFont, TextInputValue};
use engine::{
    actors::ghost::{GhostResources, Handed, OrbitParticle},
    effects::mesh_particles::MeshParticleEmitter,
};
use util::{iterators::even_distribution_on_sphere, lerp, LocalRepeatingTimer};

use ui_core::{get_large_text_style, get_text_style, ButtonColors, TRANSLUCENT_PANEL_BACKGROUND};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MenuState>()
            .init_state::<SplashScreenState>()
            .add_event::<SpawnMainMenuGhostEvent>()
            .add_systems(OnEnter(GameState::Menu), menu_entered)
            .add_systems(
                OnTransition {
                    exited: GameState::Setup,
                    entered: GameState::Menu,
                },
                go_to_splash_screen,
            )
            .add_systems(
                OnTransition {
                    exited: GameState::Game,
                    entered: GameState::Menu,
                },
                go_to_main_screen,
            )
            .add_systems(
                OnTransition {
                    exited: GameState::GameOver,
                    entered: GameState::Menu,
                },
                go_to_main_screen,
            )
            .add_systems(OnEnter(MenuState::SplashScreen), splash_screen_entered)
            .add_systems(OnEnter(SplashScreenState::Hidden), hide_splash_screen)
            .add_systems(OnEnter(SplashScreenState::Shown), show_splash_screen)
            .add_systems(
                Update,
                exit_splash_screen.run_if(in_state(MenuState::SplashScreen)),
            )
            .add_systems(OnEnter(MenuState::Main), show_main_screen)
            .add_systems(OnExit(MenuState::Main), hide_main_screen)
            .add_systems(OnEnter(MenuState::WorldSelect), show_world_select_screen)
            .add_systems(OnExit(MenuState::WorldSelect), hide_world_select_screen)
            .add_systems(Update, spawn_ghost.run_if(in_state(GameState::Menu)));
    }
}

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
enum MenuState {
    #[default]
    Hidden,
    SplashScreen,
    Main,
    WorldSelect,
}

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
enum SplashScreenState {
    #[default]
    Hidden,
    Shown,
}

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct MenuRoot;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct SplashScreenContainer;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct MainMenuContainer;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct MainMenuElement;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct MainMenuGhost;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WorldSelectContainer;

#[derive(Component, Clone)]
#[component(storage = "SparseSet")]
struct WorldSelectWorldButton {
    world_name: String,
}

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WorldSelectCreateText;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WorldSelectLoadLevelContainer;

#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct WorldSelectLoadLevelButton(&'static str);

#[derive(Event)]
struct SpawnMainMenuGhostEvent {
    transform: Transform,
    handed: Handed,
}

fn menu_entered(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("setting up menu...");
    setup_splash_screen(&mut commands, &asset_server);
    setup_main_screen(&mut commands, &asset_server);
    setup_world_select_screen(&mut commands, &asset_server);
}

fn setup_splash_screen(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands
        .spawn((
            StateScoped(GameState::Menu),
            SplashScreenContainer,
            MenuRoot,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
            PickingBehavior::IGNORE,
        ))
        .with_children(|sections| {
            sections.spawn((
                Text("Angry Pie".into()),
                get_large_text_style(asset_server).clone(),
            ));
        });
}

fn setup_main_screen(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    let button = (
        ButtonColors::default(),
        Node {
            width: Val::Percent(100.),
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
    );
    commands
        .spawn((
            StateScoped(GameState::Menu),
            MainMenuContainer,
            MenuRoot,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            Visibility::Hidden,
            PickingBehavior::IGNORE,
        ))
        .with_children(|sections| {
            sections
                .spawn((Node {
                    height: Val::Percent(25.),
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|logo| {
                    logo.spawn((
                        Text("Wisphaven".into()),
                        TextLayout::new_with_justify(JustifyText::Center),
                        get_large_text_style(asset_server).clone(),
                    ))
                    .observe(
                        |over: Trigger<Pointer<Down>>, mut texts: Query<&mut TextColor>| {
                            let mut color = texts.get_mut(over.entity()).unwrap();
                            color.0 = bevy::color::palettes::tailwind::CYAN_400.into();
                        },
                    );
                });
            sections
                .spawn((Node {
                    height: Val::Percent(75.),
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },))
                .with_children(|columns| {
                    columns
                        .spawn(Node {
                            height: Val::Percent(100.),
                            width: Val::Px(256.),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|buttons| {
                            buttons
                                .spawn((Name::new("play button"), button.clone()))
                                .observe(play_clicked)
                                .with_children(|text| {
                                    text.spawn((
                                        Text("Play".into()),
                                        get_text_style(asset_server).clone(),
                                    ));
                                });
                            // todo - add settings menu
                            // buttons
                            //     .spawn((Name::new("settings button"), button.clone()))
                            //     .observe(settings_clicked)
                            //     .with_children(|text| {
                            //         text.spawn((
                            //             Text("Settings".into()),
                            //             get_text_style(asset_server).clone(),
                            //         ));
                            //     });
                            buttons
                                .spawn((Name::new("quit button"), button.clone()))
                                .observe(quit_clicked)
                                .with_children(|text| {
                                    text.spawn((
                                        Text("Quit".into()),
                                        get_text_style(asset_server).clone(),
                                    ));
                                });
                        });
                });
        });
}

fn setup_world_select_screen(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    let button = (
        ButtonColors::default(),
        Node {
            width: Val::Percent(100.),
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
    );
    commands
        .spawn((
            StateScoped(GameState::Menu),
            WorldSelectContainer,
            MenuRoot,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            Visibility::Hidden,
            PickingBehavior::IGNORE,
        ))
        .with_children(|sections| {
            sections
                // create world stuff
                .spawn((Node {
                    height: Val::Percent(25.),
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },))
                .with_children(|columns| {
                    columns
                        .spawn((Node {
                            height: Val::Percent(100.),
                            width: Val::Px(512.),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },))
                        .with_children(|rows| {
                            rows.spawn((
                                Node {
                                    width: Val::Percent(100.),
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::Start,
                                    ..default()
                                },
                                BackgroundColor(TRANSLUCENT_PANEL_BACKGROUND),
                            ))
                            .with_children(|items| {
                                // text input
                                let (color, font, picking) = get_text_style(asset_server);
                                items.spawn((
                                    Node {
                                        width: Val::Percent(100.),
                                        border: UiRect::all(Val::Px(5.0)),
                                        padding: UiRect::all(Val::Px(5.0)),
                                        ..default()
                                    },
                                    BorderColor::default(),
                                    BackgroundColor::default(),
                                    TextInput,
                                    TextInputTextColor(color),
                                    TextInputTextFont(font),
                                    picking,
                                    WorldSelectCreateText,
                                ));
                                // create button
                                items
                                    .spawn(button.clone())
                                    .observe(create_clicked)
                                    .with_children(|text| {
                                        text.spawn((
                                            Text("Create".into()),
                                            get_text_style(asset_server).clone(),
                                        ));
                                    });
                            });
                        });
                });
            //load world section
            sections.spawn((
                Node {
                    height: Val::Percent(75.),
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    justify_content: JustifyContent::FlexStart,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(TRANSLUCENT_PANEL_BACKGROUND),
                WorldSelectLoadLevelContainer,
            ));
        });
    commands.run_system_cached(spawn_world_select_items);
}

fn spawn_world_select_items(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    saved_worlds: Res<SavedLevels>,
    container_query: Query<Entity, With<WorldSelectLoadLevelContainer>>,
) {
    let Ok(root) = container_query.get_single() else {
        error!("Invalid world select container");
        return;
    };
    let Some(mut root_ec) = commands.get_entity(root) else {
        error!("world select container doesn't have commands");
        return;
    };
    let button = (
        ButtonColors::default(),
        Node {
            width: Val::Percent(100.),
            height: Val::Px(48.0),
            border: UiRect::all(Val::Px(2.0)),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            margin: UiRect::all(Val::Px(4.)),
            ..default()
        },
        PickingBehavior {
            should_block_lower: false,
            ..default()
        },
        BorderColor(ButtonColors::default().default_border),
        BackgroundColor(ButtonColors::default().default_background),
        Button,
    );

    for level in saved_worlds.0.iter() {
        root_ec.with_children(|container| {
            container
                .spawn((
                    Node {
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Start,
                        ..default()
                    },
                    BackgroundColor(TRANSLUCENT_PANEL_BACKGROUND),
                    PickingBehavior {
                        should_block_lower: false,
                        ..default()
                    },
                ))
                .with_children(|components| {
                    components
                        .spawn((WorldSelectLoadLevelButton(level.name), button.clone()))
                        .observe(load_level_clicked)
                        .with_children(|text| {
                            text.spawn((
                                Text(level.name.into()),
                                get_text_style(&asset_server).clone(),
                            ));
                        });
                });
        });
    }
}

fn go_to_splash_screen(mut next_state: ResMut<NextState<MenuState>>) {
    next_state.set(MenuState::SplashScreen);
}

fn splash_screen_entered(mut next_state: ResMut<NextState<SplashScreenState>>) {
    next_state.set(SplashScreenState::Shown);
}

fn exit_splash_screen(
    mut timer: Local<LocalRepeatingTimer<1000>>,
    time: Res<Time>,
    mut next_menu_state: ResMut<NextState<MenuState>>,
    mut next_splash_state: ResMut<NextState<SplashScreenState>>,
    splash_screen_state: Res<State<SplashScreenState>>,
) {
    timer.tick(time.delta());
    if timer.finished() {
        timer.reset();
        match splash_screen_state.get() {
            SplashScreenState::Hidden => next_menu_state.set(MenuState::Main),
            SplashScreenState::Shown => {
                next_splash_state.set(SplashScreenState::Hidden);
                next_menu_state.set(MenuState::Main)
            }
        }
    }
}

fn show_splash_screen(mut container_query: Query<&mut Visibility, With<SplashScreenContainer>>) {
    for mut vis in container_query.iter_mut() {
        *vis = Visibility::Inherited;
    }
}

fn hide_splash_screen(mut container_query: Query<&mut Visibility, With<SplashScreenContainer>>) {
    for mut vis in container_query.iter_mut() {
        *vis = Visibility::Hidden;
    }
}

fn go_to_main_screen(mut next_state: ResMut<NextState<MenuState>>) {
    next_state.set(MenuState::Main);
    info!("going to main screen");
}

fn show_main_screen(
    mut container_query: Query<&mut Visibility, With<MainMenuContainer>>,
    ghost_query: Query<(), With<MainMenuGhost>>,
    mut ghost_spawner: EventWriter<SpawnMainMenuGhostEvent>,
) {
    info!("in show_main_screen");
    for mut vis in container_query.iter_mut() {
        *vis = Visibility::Visible;
        info!("it has been set to visible")
    }
    if ghost_query.is_empty() {
        ghost_spawner.send(SpawnMainMenuGhostEvent {
            transform: Transform::from_translation(Vec3::new(100.7, 99.6, 97.))
                .with_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 2.5, 0.3)),
            handed: Handed::Right,
        });
    }
}

fn hide_main_screen(mut container_query: Query<&mut Visibility, With<MainMenuContainer>>) {
    for mut vis in container_query.iter_mut() {
        *vis = Visibility::Hidden;
    }
}

fn show_world_select_screen(
    mut container_query: Query<&mut Visibility, With<WorldSelectContainer>>,
) {
    for mut vis in container_query.iter_mut() {
        *vis = Visibility::Inherited;
    }
}

fn hide_world_select_screen(
    mut container_query: Query<&mut Visibility, With<WorldSelectContainer>>,
) {
    for mut vis in container_query.iter_mut() {
        *vis = Visibility::Hidden;
    }
}

fn create_clicked(
    mut click: Trigger<Pointer<Click>>,
    text_value: Query<&TextInputValue, With<WorldSelectCreateText>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_menu_state: ResMut<NextState<MenuState>>,
    mut level_name: ResMut<LevelCreationInput>,
) {
    let input_name = &text_value.get_single().unwrap().0;
    println!("{} was clicked. Textbox has {}", click.entity(), input_name);
    click.propagate(false);
    start_level(
        input_name.clone().leak(),
        &mut level_name,
        &mut next_game_state,
        &mut next_menu_state,
    );
}

fn load_level_clicked(
    mut click: Trigger<Pointer<Click>>,
    button_query: Query<&WorldSelectLoadLevelButton>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_menu_state: ResMut<NextState<MenuState>>,
    mut level_name: ResMut<LevelCreationInput>,
) {
    let input_name = button_query.get(click.entity()).unwrap().0;
    println!(
        "{} was clicked. Loading level {}",
        click.entity(),
        input_name,
    );
    click.propagate(false);
    start_level(
        input_name,
        &mut level_name,
        &mut next_game_state,
        &mut next_menu_state,
    );
}

fn start_level(
    name: &'static str,
    level_name: &mut ResMut<LevelCreationInput>,
    next_game_state: &mut ResMut<NextState<GameState>>,
    next_menu_state: &mut ResMut<NextState<MenuState>>,
) {
    level_name.name = name;
    next_game_state.set(GameState::Game);
    next_menu_state.set(MenuState::default());
}

fn play_clicked(
    mut click: Trigger<Pointer<Click>>,
    mut next_menu_state: ResMut<NextState<MenuState>>,
) {
    println!("{} was clicked", click.entity());
    click.propagate(false);
    next_menu_state.set(MenuState::WorldSelect);
}

fn settings_clicked(mut click: Trigger<Pointer<Click>>) {
    println!("{} was clicked", click.entity());
    click.propagate(false);
    info!("settings clicked!");
}

fn quit_clicked(mut click: Trigger<Pointer<Click>>, mut exit: EventWriter<AppExit>) {
    println!("{} was clicked", click.entity());
    click.propagate(false);
    exit.send(AppExit::Success);
}

fn spawn_ghost(
    mut commands: Commands,
    res: Res<GhostResources>,
    mut spawn_requests: EventReader<SpawnMainMenuGhostEvent>,
) {
    const MIN_PARTICLE_SIZE: f32 = 0.225;
    const MAX_PARTICLE_SIZE: f32 = 0.5;
    const MIN_PARTICLE_DIST: f32 = 0.15;
    const MAX_PARTICLE_DIST: f32 = 0.25;
    const MIN_PARTICLE_SPEED: f32 = 0.05;
    const MAX_PARTICLE_SPEED: f32 = 0.15;
    const GHOST_PARTICLE_COUNT: u32 = 7;
    for spawn in spawn_requests.read() {
        let ghost_entity = commands
            .spawn((
                StateScoped(GameState::Menu),
                spawn.transform,
                Visibility::default(),
                Name::new("ghost"),
                MainMenuGhost,
            ))
            .with_children(|children| {
                //orbit particles
                for (i, point) in
                    (0..GHOST_PARTICLE_COUNT).zip(even_distribution_on_sphere(GHOST_PARTICLE_COUNT))
                {
                    //size and distance are inversely correlated
                    let size = lerp(
                        MAX_PARTICLE_SIZE,
                        MIN_PARTICLE_SIZE,
                        i as f32 / GHOST_PARTICLE_COUNT as f32,
                    );
                    let dist = lerp(
                        MIN_PARTICLE_DIST,
                        MAX_PARTICLE_DIST,
                        i as f32 / GHOST_PARTICLE_COUNT as f32,
                    );
                    let speed = lerp(
                        MIN_PARTICLE_SPEED,
                        MAX_PARTICLE_SPEED,
                        i as f32 / GHOST_PARTICLE_COUNT as f32,
                    );
                    let material = res.particle_materials[i as usize].clone();
                    let angle_inc = 2.0 * std::f32::consts::PI / GHOST_PARTICLE_COUNT as f32;
                    let angle = i as f32 * angle_inc;
                    children.spawn((
                        MeshMaterial3d(material),
                        Mesh3d(res.particle_mesh.clone()),
                        Transform::from_translation(point * dist).with_scale(Vec3::splat(size)),
                        OrbitParticle::stable(
                            dist,
                            Vec3::new(speed * angle.sin(), 0.0, speed * angle.cos()),
                        ),
                    ));
                }
            })
            .id();
        //right hand
        let right_hand_entity = spawn_ghost_hand(
            ghost_entity,
            spawn.transform,
            Vec3::new(0.45, 0.2, -0.9),
            Vec3::new(0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &res,
            &mut commands,
        );
        //left hand
        let left_hand_entity = spawn_ghost_hand(
            ghost_entity,
            spawn.transform,
            Vec3::new(-0.75, 0.2, -0.9),
            Vec3::new(-0.6, 0.2, -0.5),
            0.15,
            Quat::default(),
            &res,
            &mut commands,
        );
        spawn.handed.assign_hands(
            ghost_entity,
            left_hand_entity,
            right_hand_entity,
            &mut commands,
        );

        //falling particles
        commands.spawn((
            StateScoped(GameState::Menu),
            Name::new("emitter"),
            Transform::from_translation(
                spawn.transform.translation - spawn.transform.scale.y * 3. * Vec3::Y,
            ),
            Visibility::default(),
            MeshParticleEmitter {
                shape: engine::effects::mesh_particles::MeshParticleShape::Cube,
                min_scale: 0.08 * Vec3::ONE,
                max_scale: 0.125 * Vec3::ONE,
                emit_radius: 1.,
                speed: 0.05,
                gravity_mult: -0.0625,
                drag: 0.025,
                lifetime: Duration::from_secs(10),
                spawn_count_min: 3,
                spawn_count_max: 6,
                repeat_time: Some(Duration::from_secs_f32(0.2)),
                min_color: Vec3::new(57., 42., 84.) / 255.,
                max_color: Vec3::new(73., 74., 117.) / 255.,
                ..default()
            },
            MainMenuElement,
        ));
    }
}

pub fn spawn_ghost_hand(
    owner: Entity,
    owner_pos: Transform,
    offset: Vec3,
    windup_offset: Vec3,
    hand_size: f32,
    hand_rot: Quat,
    res: &GhostResources,
    commands: &mut Commands,
) -> Entity {
    const HAND_PARTICLE_COUNT: u32 = 3;
    let min_particle_size: f32 = 0.1 / hand_size;
    let max_particle_size: f32 = 0.15 / hand_size;
    let min_particle_speed: f32 = 0.05 / hand_size;
    let max_particle_speed: f32 = 0.1 / hand_size;
    let min_particle_dist: f32 = 0.15 / hand_size;
    let max_particle_dist: f32 = 0.2 / hand_size;
    commands
        .spawn((
            StateScoped(GameState::Menu),
            Mesh3d(res.particle_mesh.clone()),
            MeshMaterial3d(res.hand_particle_material.clone()),
            Transform::from_translation(owner_pos.transform_point(offset))
                .with_scale(Vec3::splat(hand_size)),
            Hand {
                owner,
                offset,
                windup_offset,
                scale: hand_size,
                rotation: hand_rot,
                state: HandState::Following,
            },
        ))
        .with_children(|children| {
            //orbit particles
            for (i, point) in
                (0..HAND_PARTICLE_COUNT).zip(even_distribution_on_sphere(HAND_PARTICLE_COUNT))
            {
                //size and distance are inversely correlated
                let size = lerp(
                    max_particle_size,
                    min_particle_size,
                    i as f32 / HAND_PARTICLE_COUNT as f32,
                );
                let dist = lerp(
                    min_particle_dist,
                    max_particle_dist,
                    i as f32 / HAND_PARTICLE_COUNT as f32,
                );
                let speed = lerp(
                    min_particle_speed,
                    max_particle_speed,
                    i as f32 / HAND_PARTICLE_COUNT as f32,
                );
                let material = res.hand_particle_material.clone();
                let angle_inc = 2.0 * std::f32::consts::PI / HAND_PARTICLE_COUNT as f32;
                let angle = i as f32 * angle_inc;
                children.spawn((
                    MeshMaterial3d(material),
                    Mesh3d(res.particle_mesh.clone()),
                    Transform::from_translation(point * dist).with_scale(Vec3::splat(size)),
                    OrbitParticle::stable(
                        dist,
                        Vec3::new(speed * angle.sin(), 0.0, speed * angle.cos()),
                    ),
                ));
            }
        })
        .id()
}
