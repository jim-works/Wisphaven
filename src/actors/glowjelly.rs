use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::physics::PhysicsObjectBundle;

use super::CombatInfo;

#[derive(Resource)]
pub struct GlowjellyResources {
    pub anim: Handle<AnimationClip>,
    pub scene: Handle<Scene>,
}

#[derive(Component)]
pub struct Glowjelly;

pub struct SpawnGlowjellyEvent {
    pub location: Transform,
    pub color: Color,
}

pub struct GlowjellyPlugin;

impl Plugin for GlowjellyPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(load_resources)
            .add_system(spawn_glowjelly)
            .add_system(keyboard_animation_control)
            .add_event::<SpawnGlowjellyEvent>();
    }
}

pub fn load_resources(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(GlowjellyResources {
        anim: assets.load("glowjelly/glowjelly.gltf#Animation0"),
        scene: assets.load("glowjelly/glowjelly.gltf#Scene0"),
    });
}

pub fn spawn_glowjelly(
    mut commands: Commands,
    jelly_res: Res<GlowjellyResources>,
    mut spawn_requests: EventReader<SpawnGlowjellyEvent>,
) {
    for spawn in spawn_requests.iter() {
        commands
            .spawn((
                SceneBundle {
                    scene: jelly_res.scene.clone_weak(),
                    transform: spawn.location,
                    ..default()
                },
                Name::new("glowjelly"),
                CombatInfo::new(100.0, 100.0),
                PhysicsObjectBundle {
                    rigidbody: RigidBody::Dynamic,
                    collider: Collider::cuboid(0.5, 0.5, 0.5),
                    ..default()
                },
                GravityScale(0.1),
                Glowjelly,
            ))
            .with_children(|cb| {
                cb.spawn(PointLightBundle {
                    point_light: PointLight {
                        color: spawn.color,
                        intensity: 500.0,
                        shadows_enabled: true,
                        ..default()
                    },
                    ..default()
                });
            });
    }
}

fn keyboard_animation_control(
    keyboard_input: Res<Input<KeyCode>>,
    jelly_anim: Res<GlowjellyResources>,
    mut animation_player: Query<&mut AnimationPlayer>,
    mut impulse_query: Query<&mut ExternalImpulse, With<Glowjelly>>,
) {
    if keyboard_input.just_pressed(KeyCode::Return) {
        for mut anim_player in animation_player.iter_mut() {
            anim_player.start(jelly_anim.anim.clone_weak());
            println!("resuming!");
        }
        for mut impulse in impulse_query.iter_mut() {
            impulse.impulse += Vec3::new(0.0, 5.0, 0.0);
        }
    }
}
