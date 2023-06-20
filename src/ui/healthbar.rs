use std::f32::consts::PI;

use bevy_mod_billboard::prelude::*;

use bevy::prelude::{shape::Quad, *};

use crate::{actors::CombatInfo, controllers::PlayerActionOrigin};

pub struct HealthbarPlugin;

impl Plugin for HealthbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(init)
            .add_system(update_healthbar)
            .add_system(follow_billboard);
    }
}

#[derive(Component)]
pub struct Healthbar {
    tracking: Entity,
}

#[derive(Component)]
pub struct BillboardFollow {
    offset: Vec3,
    tracking: Entity,
}

#[derive(Component)]
pub struct HealthbarBackground;

#[derive(Resource)]
pub struct HealthbarResources {
    foreground_image: Handle<Image>,
    background_image: Handle<Image>,
    foreground_billboard_texture: Handle<BillboardTexture>,
    background_billboard_texture: Handle<BillboardTexture>,
    billboard_mesh: Handle<Mesh>,
}

fn init(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut billboard_textures: ResMut<Assets<BillboardTexture>>,
) {
    let fg: Handle<Image> = assets.load("textures/HealthbarForeground.png").into();
    let bg: Handle<Image> = assets.load("textures/HealthbarBackground.png").into();
    commands.insert_resource(HealthbarResources {
        foreground_image: fg.clone(),
        background_image: bg.clone(),
        foreground_billboard_texture: billboard_textures.add(BillboardTexture::Single(fg)),
        background_billboard_texture: billboard_textures.add(BillboardTexture::Single(bg)),
        billboard_mesh: meshes.add(Quad::new(Vec2::new(1.0, 0.25)).into()),
    });
}

pub fn spawn_billboard_healthbar(
    commands: &mut Commands,
    healthbar_resources: &Res<HealthbarResources>,
    tracking: Entity,
    offset: Vec3,
) -> Entity {
    //parent
    commands
        .spawn((
            SpatialBundle::default(),
            Name::new("Healthbar"),
            BillboardFollow { offset, tracking },
        ))
        .with_children(|children| {
            //foreground
            children.spawn((
                BillboardTextureBundle {
                    texture: healthbar_resources.foreground_billboard_texture.clone(),
                    mesh: healthbar_resources.billboard_mesh.clone().into(),
                    ..default()
                },
                Healthbar { tracking },
            ));
            children.spawn((
                BillboardTextureBundle {
                    //background should be behind foreground
                    transform: Transform::from_xyz(0.0, 0.0, 0.01),
                    texture: healthbar_resources.background_billboard_texture.clone(),
                    mesh: healthbar_resources.billboard_mesh.clone().into(),
                    ..default()
                },
                HealthbarBackground,
            ));
        })
        .id()
}

fn update_healthbar(
    mut healthbars: Query<(&Healthbar, &mut Transform)>,
    info_query: Query<&CombatInfo>,
) {
    for (healthbar, mut tf) in healthbars.iter_mut() {
        if let Ok(info) = info_query.get(healthbar.tracking) {
            let scale_factor = if info.max_health == 0.0 {
                1.0
            } else {
                info.curr_health / info.max_health
            };
            tf.scale.x = scale_factor.clamp(0.0, 1.0);
        }
    }
}

fn follow_billboard(
    mut commands: Commands,
    mut billboards: Query<(Entity, &mut Transform, &BillboardFollow)>,
    target_query: Query<&GlobalTransform>,
    camera_query: Query<&GlobalTransform, With<PlayerActionOrigin>>,
) {
    if let Ok(cam_tf) = camera_query.get_single() {
        for (entity, mut tf, follow) in billboards.iter_mut() {
            if let Ok(target_tf) = target_query.get(follow.tracking) {
                tf.translation = target_tf.translation() + follow.offset;
                tf.look_at(cam_tf.translation(), cam_tf.up());
            } else {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
