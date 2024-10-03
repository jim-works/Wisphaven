use bevy::{
    core_pipeline::Skybox,
    prelude::*,
    render::{camera::CameraProjection, primitives::Frustum},
};

use crate::{
    actors::LocalPlayer,
    effects::camera::CameraEffectsBundle,
    world::atmosphere::{LoadingSkyboxCubemap, SkyboxCubemap},
    GameState,
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        let cam = app
            .world_mut()
            .spawn((
                Camera3dBundle {
                    transform: Transform::from_xyz(0.0, 0.3, 0.0),
                    ..default()
                },
                CameraEffectsBundle::default(),
            ))
            .id();
        app.insert_resource(MainCamera(cam));
        app.add_systems(Update, follow_player.run_if(in_state(GameState::Game)))
            .add_systems(OnEnter(GameState::Game), on_enter_game)
            .add_systems(OnEnter(GameState::Menu), on_enter_menu);
    }
}

#[derive(Resource)]
pub struct MainCamera(pub Entity);

fn on_enter_game(
    mut commands: Commands,
    main_camera: Res<MainCamera>,
    skybox: Option<Res<SkyboxCubemap>>,
    loading_skybox: Option<Res<LoadingSkyboxCubemap>>,
) {
    let Some(mut ec) = commands.get_entity(main_camera.0) else {
        error!("main camera doesn't exist!");
        return;
    };
    let projection = PerspectiveProjection {
        fov: std::f32::consts::PI / 2.,
        far: 1_000_000_000.0,
        ..default()
    };
    let sky_image = skybox.map(|sky| sky.0.clone()).unwrap_or(
        loading_skybox
            .expect("there was no skybox or loading skybox when populating the camera")
            .0
            .clone(),
    );
    ec.insert((
        // placeholder brightness - actually set by atmosphere
        Skybox {
            image: sky_image,
            brightness: 750.,
        },
        // placeholder values - actually set by atmosphere
        FogSettings {
            color: Color::srgba(0.56, 0.824, 1.0, 1.0),
            // directional_light_color: Color::srgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 0.8,
            falloff: FogFalloff::Linear {
                start: 100.0,
                end: 200.0,
            },
            ..default()
        },
        Projection::Perspective(projection.clone()),
        Frustum::from_clip_from_world(&projection.get_clip_from_view()),
    ));
}

fn on_enter_menu(mut commands: Commands, main_camera: Res<MainCamera>) {
    //offset menu from world for now
    const CAMERA_TF: Transform = Transform::from_translation(Vec3::new(100., 100., 100.));
    let Some(mut ec) = commands.get_entity(main_camera.0) else {
        error!("main camera doesn't exist");
        return;
    };
    ec.insert((
        CAMERA_TF.clone(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Projection::default(),
        Frustum::default(),
    ));
    ec.remove::<(Skybox, FogSettings)>();
}

fn follow_player(
    player_query: Query<&GlobalTransform, With<LocalPlayer>>,
    mut update_query: Query<&mut Transform, Without<LocalPlayer>>,
    camera: Res<MainCamera>,
) {
    for gtf in player_query.iter() {
        if let Ok(mut tf) = update_query.get_mut(camera.0) {
            *tf = gtf.compute_transform();
        }
    }
}