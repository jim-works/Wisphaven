use std::time::Duration;

use bevy::{app::AppExit, ecs::system::SystemId, prelude::*};

use engine::{
    actors::ghost::{GhostResources, Hand, HandState, Handed, OrbitParticle, SwingHand, UseHand},
    effects::mesh_particles::MeshParticleEmitter,
    GameState,
};
use util::{iterators::even_distribution_on_sphere, lerp, LocalRepeatingTimer};

use crate::{
    styles,
    third_party::bevy_text_edit::{TextEditFocus, TextEditable},
};

use super::{
    styles::{get_large_text_style, get_text_style},
    ButtonAction, ButtonColors,
};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        let system_ids = MainMenuSystemIds {
            play_click: app.world_mut().register_system(play_clicked),
            settings_click: app.world_mut().register_system(settings_clicked),
            quit_click: app.world_mut().register_system(quit_clicked),
            create_click: app.world_mut().register_system(create_clicked),
        };
        app.init_state::<MenuState>()
            .init_state::<SplashScreenState>()
            .add_event::<SpawnMainMenuGhostEvent>()
            .insert_resource(system_ids)
            .add_systems(OnEnter(GameState::Menu), menu_entered)
            .add_systems(OnExit(GameState::Menu), menu_exited)
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

#[derive(Event)]
struct SpawnMainMenuGhostEvent {
    transform: Transform,
    handed: Handed,
}

#[derive(Resource)]
struct MainMenuSystemIds {
    play_click: SystemId,
    settings_click: SystemId,
    quit_click: SystemId,
    create_click: SystemId,
}

fn menu_entered(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    res: Res<MainMenuSystemIds>,
) {
    setup_splash_screen(&mut commands, &asset_server);
    setup_main_screen(&mut commands, &asset_server, &res);
    setup_world_select_screen(&mut commands, &asset_server, &res);
}

fn menu_exited(
    mut commands: Commands,
    menu_query: Query<Entity, Or<(With<MenuRoot>, With<MainMenuElement>)>>,
    ghost_query: Query<(Entity, &SwingHand, &UseHand), With<MainMenuGhost>>,
) {
    for entity in menu_query.iter() {
        if let Some(ec) = commands.get_entity(entity) {
            ec.despawn_recursive();
        }
    }
    for (entity, hand1, hand2) in ghost_query.iter() {
        let entities = [entity, hand1.hand, hand2.hand];
        for e in entities {
            if let Some(ec) = commands.get_entity(e) {
                ec.despawn_recursive();
            }
        }
    }
}

fn setup_splash_screen(commands: &mut Commands, asset_server: &Res<AssetServer>) {
    commands
        .spawn((
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
        ))
        .with_children(|sections| {
            sections.spawn((
                Text("Angry Pie".into()),
                get_large_text_style(asset_server).clone(),
            ));
        });
}

fn setup_main_screen(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    res: &Res<MainMenuSystemIds>,
) {
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
                    ));
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
                                .spawn((ButtonAction::new(res.play_click), button.clone()))
                                .with_children(|text| {
                                    text.spawn((
                                        Text("Play".into()),
                                        get_text_style(asset_server).clone(),
                                    ));
                                });
                            buttons
                                .spawn((ButtonAction::new(res.settings_click), button.clone()))
                                .with_children(|text| {
                                    text.spawn((
                                        Text("Settings".into()),
                                        get_text_style(asset_server).clone(),
                                    ));
                                });
                            buttons
                                .spawn((ButtonAction::new(res.quit_click), button.clone()))
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

fn setup_world_select_screen(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    res: &Res<MainMenuSystemIds>,
) {
    // todo 0.15 - when 0.15 comes out, add scrolling support
    // https://github.com/bevyengine/bevy/pull/15291
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
                        get_large_text_style(asset_server).clone(),
                    ));
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
                                BackgroundColor(styles::TRANSLUCENT_PANEL_BACKGROUND),
                            ))
                            .with_children(|items| {
                                // text input
                                items.spawn((
                                    Text("New World".into()),
                                    get_text_style(asset_server).clone(),
                                    TextEditable::default(),
                                    Interaction::None,
                                    WorldSelectCreateText,
                                ));
                                // create button
                                items
                                    .spawn((ButtonAction::new(res.create_click), button.clone()))
                                    .with_children(|text| {
                                        text.spawn((
                                            Text("Create".into()),
                                            get_text_style(asset_server).clone(),
                                        ));
                                    });
                            });
                        });
                });
        });
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
}

fn show_main_screen(
    mut container_query: Query<&mut Visibility, With<MainMenuContainer>>,
    ghost_query: Query<(), With<MainMenuGhost>>,
    mut ghost_spawner: EventWriter<SpawnMainMenuGhostEvent>,
) {
    for mut vis in container_query.iter_mut() {
        *vis = Visibility::Visible;
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
    text_query: Query<Entity, With<WorldSelectCreateText>>,
    mut commands: Commands,
) {
    for mut vis in container_query.iter_mut() {
        *vis = Visibility::Inherited;
    }
    for entity in text_query.iter() {
        if let Some(mut ec) = commands.get_entity(entity) {
            ec.try_insert(TextEditFocus);
        }
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
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_menu_state: ResMut<NextState<MenuState>>,
) {
    next_game_state.set(GameState::Game);
    next_menu_state.set(MenuState::default());
}

fn play_clicked(mut next_menu_state: ResMut<NextState<MenuState>>) {
    next_menu_state.set(MenuState::WorldSelect);
}

fn settings_clicked() {
    info!("settings clicked!");
}

fn quit_clicked(mut exit: EventWriter<AppExit>) {
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
                    let material = (&res.particle_materials[i as usize]).clone();
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
