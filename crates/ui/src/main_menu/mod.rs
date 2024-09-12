use std::time::Duration;

use bevy::{
    app::AppExit, core_pipeline::clear_color::ClearColorConfig, ecs::system::SystemId, prelude::*,
};

use engine::{
    actors::ghost::{GhostResources, Hand, HandState, Handed, OrbitParticle, SwingHand, UseHand},
    effects::mesh_particles::MeshParticleEmitter,
    physics::movement::{Drag, GravityMult},
    GameState,
};
use util::{iterators::even_distribution_on_sphere, lerp, LocalRepeatingTimer};

use super::{
    styles::{get_large_text_style, get_text_style},
    ButtonAction, ButtonColors,
};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        let system_ids = MainMenuSystemIds {
            play_click: app.world.register_system(play_clicked),
            settings_click: app.world.register_system(settings_clicked),
            quit_click: app.world.register_system(quit_clicked),
        };
        app.add_state::<MenuState>()
            .add_state::<SplashScreenState>()
            .add_event::<SpawnMainMenuGhostEvent>()
            .insert_resource(system_ids)
            .add_systems(OnEnter(GameState::Menu), menu_entered)
            .add_systems(OnExit(GameState::Menu), menu_exited)
            .add_systems(
                OnTransition {
                    from: GameState::Setup,
                    to: GameState::Menu,
                },
                go_to_splash_screen,
            )
            .add_systems(
                OnTransition {
                    from: GameState::Game,
                    to: GameState::Menu,
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
struct MenuCamera;

#[derive(Component, Clone, Copy)]
struct MenuRoot;

#[derive(Component, Clone, Copy)]
struct SplashScreenContainer;

#[derive(Component, Clone, Copy)]
struct SplashScreenElement;

#[derive(Component, Clone, Copy)]
struct MainMenuContainer;

#[derive(Component, Clone, Copy)]
struct MainMenuElement;

#[derive(Component, Clone, Copy)]
struct MainMenuGhost;

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
}

fn menu_entered(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    res: Res<MainMenuSystemIds>,
) {
    commands.spawn((
        MenuCamera,
        Camera3dBundle {
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            transform: CAMERA_TF.clone(),
            ..default()
        },
    ));
    setup_splash_screen(&mut commands, &asset_server);
    setup_main_screen(&mut commands, &asset_server, &res);
    setup_world_select_screen(&mut commands);
}

//there is a bright flash that shows up at spawn for some reason and I'm too lazy to figure it out rn
const CAMERA_TF: Transform = Transform::from_translation(Vec3::new(100., 100., 100.));

fn menu_exited(
    mut commands: Commands,
    camera_query: Query<Entity, With<MenuCamera>>,
    menu_query: Query<Entity, With<MenuRoot>>,
    ghost_query: Query<(Entity, &SwingHand, &UseHand), With<MainMenuGhost>>,
) {
    for entity in camera_query.iter() {
        if let Some(ec) = commands.get_entity(entity) {
            ec.despawn_recursive();
        }
    }
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
            SplashScreenElement,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                visibility: Visibility::Hidden,
                ..default()
            },
        ))
        .with_children(|sections| {
            sections.spawn((
                SplashScreenElement,
                TextBundle {
                    text: Text {
                        sections: vec![TextSection {
                            value: "Angry Pie".into(),
                            style: get_large_text_style(asset_server),
                        }],
                        ..default()
                    },
                    ..default()
                },
            ));
        });
}

fn setup_main_screen(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    res: &Res<MainMenuSystemIds>,
) {
    let button = (
        MainMenuElement,
        ButtonColors::default(),
        ButtonBundle {
            style: Style {
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
            border_color: BorderColor(ButtonColors::default().default_border),
            background_color: BackgroundColor(ButtonColors::default().default_background),
            ..default()
        },
    );
    commands
        .spawn((
            MainMenuContainer,
            MenuRoot,
            MainMenuElement,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },
                visibility: Visibility::Hidden,
                ..default()
            },
        ))
        .with_children(|sections| {
            sections
                .spawn((
                    MainMenuElement,
                    NodeBundle {
                        style: Style {
                            height: Val::Percent(25.),
                            width: Val::Percent(100.),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|logo| {
                    logo.spawn((
                        MainMenuElement,
                        TextBundle {
                            text: Text {
                                alignment: TextAlignment::Center,
                                sections: vec![TextSection {
                                    value: "Wisphaven".into(),
                                    style: get_large_text_style(asset_server),
                                }],
                                ..default()
                            },
                            ..default()
                        },
                    ));
                });
            sections
                .spawn((
                    MainMenuElement,
                    NodeBundle {
                        style: Style {
                            height: Val::Percent(75.),
                            width: Val::Percent(100.),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::FlexStart,
                            justify_content: JustifyContent::FlexStart,
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|columns| {
                    columns
                        .spawn((
                            MainMenuElement,
                            NodeBundle {
                                style: Style {
                                    height: Val::Percent(100.),
                                    width: Val::Px(256.),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::FlexStart,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                ..default()
                            },
                        ))
                        .with_children(|buttons| {
                            buttons
                                .spawn((ButtonAction::new(res.play_click), button.clone()))
                                .with_children(|text| {
                                    text.spawn((
                                        MainMenuElement,
                                        TextBundle {
                                            text: Text {
                                                sections: vec![TextSection {
                                                    value: "Play".into(),
                                                    style: get_text_style(asset_server),
                                                }],
                                                ..default()
                                            },
                                            ..default()
                                        },
                                    ));
                                });
                            buttons
                                .spawn((ButtonAction::new(res.settings_click), button.clone()))
                                .with_children(|text| {
                                    text.spawn((
                                        MainMenuElement,
                                        TextBundle {
                                            text: Text {
                                                sections: vec![TextSection {
                                                    value: "Settings".into(),
                                                    style: get_text_style(asset_server),
                                                }],
                                                ..default()
                                            },
                                            ..default()
                                        },
                                    ));
                                });
                            buttons
                                .spawn((ButtonAction::new(res.quit_click), button.clone()))
                                .with_children(|text| {
                                    text.spawn((
                                        MainMenuElement,
                                        TextBundle {
                                            text: Text {
                                                sections: vec![TextSection {
                                                    value: "Quit".into(),
                                                    style: get_text_style(asset_server),
                                                }],
                                                ..default()
                                            },
                                            ..default()
                                        },
                                    ));
                                });
                        });
                });
        });
}

fn setup_world_select_screen(commands: &mut Commands) {
    //MenuRoot
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
            SplashScreenState::Shown => next_splash_state.set(SplashScreenState::Hidden),
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

fn show_world_select_screen() {}

fn play_clicked(
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_menu_state: ResMut<NextState<MenuState>>,
) {
    next_game_state.set(GameState::Game);
    next_menu_state.set(MenuState::default());
}

fn settings_clicked() {
    info!("settings clicked!");
}

fn quit_clicked(mut exit: EventWriter<AppExit>) {
    exit.send(AppExit);
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
                PbrBundle {
                    material: res.material.clone(),
                    transform: spawn.transform,
                    ..default()
                },
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
                        PbrBundle {
                            material,
                            mesh: res.particle_mesh.clone(),
                            transform: Transform::from_translation(point * dist)
                                .with_scale(Vec3::splat(size)),
                            ..default()
                        },
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
            SpatialBundle {
                transform: Transform::from_translation(spawn.transform.translation),
                ..default()
            },
            MeshParticleEmitter {
                shape: engine::effects::mesh_particles::MeshParticleShape::Cube,
                min_scale: Vec3::new(1., 1., 1.),
                max_scale: 5. * Vec3::ONE,
                emit_radius: 1.,
                speed: 0.,
                gravity_mult: -1.,
                drag: 0.,
                lifetime: Duration::from_secs(5),
                spawn_count_min: 1,
                spawn_count_max: 3,
                repeat_time: Some(Duration::from_secs_f32(1.)),
                ..default()
            },
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
            PbrBundle {
                mesh: res.particle_mesh.clone(),
                material: res.hand_particle_material.clone(),
                transform: Transform::from_translation(owner_pos.transform_point(offset))
                    .with_scale(Vec3::splat(hand_size)),
                ..default()
            },
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
                    PbrBundle {
                        material,
                        mesh: res.particle_mesh.clone(),
                        transform: Transform::from_translation(point * dist)
                            .with_scale(Vec3::splat(size)),
                        ..default()
                    },
                    OrbitParticle::stable(
                        dist,
                        Vec3::new(speed * angle.sin(), 0.0, speed * angle.cos()),
                    ),
                ));
            }
        })
        .id()
}
