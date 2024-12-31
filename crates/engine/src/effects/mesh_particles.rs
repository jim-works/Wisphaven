use std::time::Duration;

use bevy::prelude::*;
use rand::{thread_rng, Rng};
use util::bevy_utils::TimedDespawner;

use crate::physics::{
    collision::IgnoreTerrainCollision,
    movement::{Drag, GravityMult, Velocity},
    PhysicsBundle,
};

pub(super) struct MeshParticlesPlugin;

impl Plugin for MeshParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(Update, spawn_particles);
    }
}

#[derive(Resource)]
struct MeshParticlesResource {
    cube: Handle<Mesh>,
    material: StandardMaterial,
}

#[derive(Clone, Default, Debug)]
pub enum MeshParticleShape {
    #[default]
    Cube,
}

#[derive(Component)]
pub struct MeshParticleEmitter {
    pub shape: MeshParticleShape,
    pub min_scale: Vec3,
    pub max_scale: Vec3,
    pub emit_radius: f32,
    pub speed: f32,
    pub gravity_mult: f32,
    pub drag: f32,
    pub lifetime: Duration,
    pub spawn_count_min: u32,
    pub spawn_count_max: u32,
    pub repeat_time: Option<Duration>,
    pub min_color: Vec3,
    pub max_color: Vec3,
    pub _timer: Timer,
}

impl Default for MeshParticleEmitter {
    fn default() -> Self {
        Self {
            shape: Default::default(),
            min_scale: Vec3::ONE,
            max_scale: Vec3::ONE,
            emit_radius: 1.,
            speed: 1.,
            gravity_mult: 0.,
            drag: 0.1,
            lifetime: Duration::from_secs(1),
            spawn_count_min: 1,
            spawn_count_max: 1,
            repeat_time: Some(Duration::from_secs(1)),
            min_color: Vec3::ONE,
            max_color: Vec3::ONE,
            _timer: Timer::new(Duration::ZERO, TimerMode::Once),
        }
    }
}

fn init(mut meshes: ResMut<Assets<Mesh>>, mut commands: Commands) {
    commands.insert_resource(MeshParticlesResource {
        cube: meshes.add(util::bevy_utils::cuboid(Vec3::ONE)),
        material: StandardMaterial::default(),
    });
}

fn spawn_particles(
    mut query: Query<(&mut MeshParticleEmitter, &GlobalTransform)>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    resources: Res<MeshParticlesResource>,
    time: Res<Time>,
) {
    let dt = time.delta();
    let mut rng = thread_rng();
    for (mut emitter, gtf) in query.iter_mut() {
        emitter._timer.tick(dt);
        if !emitter._timer.finished() {
            continue;
        }
        if let Some(repeat_duration) = emitter.repeat_time {
            emitter._timer = Timer::new(repeat_duration, TimerMode::Repeating);
        }
        let count = rng.sample(rand::distributions::Uniform::new_inclusive(
            emitter.spawn_count_min,
            emitter.spawn_count_max,
        ));
        for _ in 0..count {
            let offset = util::sample_sphere_surface(&mut rng) * emitter.emit_radius;
            let scale = emitter
                .min_scale
                .lerp(emitter.max_scale, util::random_proportion(&mut rng));
            let color = emitter
                .min_color
                .lerp(emitter.max_color, util::random_proportion(&mut rng));
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(color.x, color.y, color.z),
                ..resources.material.clone()
            });
            let mesh = match emitter.shape {
                MeshParticleShape::Cube => resources.cube.clone(),
            };
            #[allow(state_scoped_entities)]
            commands.spawn((
                MeshMaterial3d(material),
                Mesh3d(mesh),
                Transform::from_translation(gtf.translation() + offset).with_scale(scale),
                TimedDespawner(Timer::new(emitter.lifetime, TimerMode::Once)),
                PhysicsBundle {
                    velocity: Velocity(offset * emitter.speed),
                    gravity: GravityMult::new(emitter.gravity_mult),
                    drag: Drag(emitter.drag),
                    ..default()
                },
                IgnoreTerrainCollision,
            ));
        }
    }
}
