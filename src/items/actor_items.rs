use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::{actors::{ActorName, ActorResources}, physics::{query::{RaycastHit, Ray, self}, collision::Aabb}, world::{BlockPhysics, Level}};

use super::{UseItemEvent, ItemSystemSet};

pub struct ActorItems;

impl Plugin for ActorItems {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, do_spawn_actors.in_set(ItemSystemSet::UsageProcessing))
            .add_systems(Startup, setup)
            .register_type::<SpawnActorItem>();
    }
}

#[derive(Resource)]
struct SpawnItemResources {
    spawn_particles: Entity,
}

#[derive(Component)]
struct SpawnParticles;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct SpawnActorItem(pub ActorName);

fn setup(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    let mut color_gradient1 = Gradient::new();
    color_gradient1.add_key(0.0, Vec4::new(0.75, 0.75, 0.75, 1.0));
    color_gradient1.add_key(0.3, Vec4::new(0.5, 0.5, 0.75, 1.0));
    color_gradient1.add_key(0.5, Vec4::new(0.15, 0.15, 0.25, 1.0));
    color_gradient1.add_key(0.7, Vec4::new(0.0, 0.0, 0.0, 0.0));

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(0.0, Vec2::splat(0.2));
    size_gradient1.add_key(0.3, Vec2::splat(0.2));
    size_gradient1.add_key(1.0, Vec2::splat(0.0));

    let writer = ExprWriter::new();

    // Give a bit of variation by randomizing the age per particle. This will
    // control the starting color and starting size of particles.
    let age = writer.lit(0.).uniform(writer.lit(0.2)).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Give a bit of variation by randomizing the lifetime per particle
    let lifetime = writer.lit(0.8).uniform(writer.lit(1.2)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Add constant downward acceleration to simulate gravity
    let accel = writer.lit(Vec3::Y * -4.).expr();
    let update_accel = AccelModifier::new(accel);

    // Add drag to make particles slow down a bit after the initial explosion
    let drag = writer.lit(5.).expr();
    let update_drag = LinearDragModifier::new(drag);

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::new(0., 0.25, 0.)).expr(),
        radius: writer.lit(0.25).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Give a bit of variation by randomizing the initial speed
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: (writer.rand(ScalarType::Float) * writer.lit(5.) + writer.lit(5.)).expr(),
    };

    let effect = EffectAsset::new(50, Spawner::once(50.0.into(), true), writer.finish())
        .with_name("spawn particles")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .update(update_drag)
        .update(update_accel)
        .render(ColorOverLifetimeModifier {
            gradient: color_gradient1,
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient1,
            screen_space_size: false,
        })
        .render(OrientModifier {
            mode: OrientMode::FaceCameraPosition
        });
    let id = commands
        .spawn((
            Name::new("spawn particle"),
            ParticleEffectBundle {
                effect: ParticleEffect::new(effects.add(effect)),
                transform: Transform::default(),
                ..Default::default()
            },
            SpawnParticles,
        ))
        .id();
    commands.insert_resource(SpawnItemResources {
        spawn_particles: id,
    });
}

fn do_spawn_actors(
    mut reader: EventReader<UseItemEvent>,
    mut commands: Commands,
    item_query: Query<&SpawnActorItem>,
    mut particles: Query<(&mut Transform, &mut EffectSpawner), With<SpawnParticles>>,
    resources: Res<ActorResources>,
    effects: Res<SpawnItemResources>,
    level: Res<Level>,
    block_physics_query: Query<&BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &Aabb)>,
) {
    const REACH: f32 = 10.0;
    const BACKWARD_DIST: f32 = 0.5;
    for UseItemEvent { user, inventory_slot: _, stack, tf } in reader.read() {
        if let Ok(item) = item_query.get(stack.id) {
            if let Some(RaycastHit::Block(_, hit)) = query::raycast(
                Ray::new(tf.translation, tf.forward(), REACH),
                &level,
                &block_physics_query,
                &object_query,
                vec![*user]
            ) {
                let impact_dist = (hit.hit_pos - tf.translation).normalize_or_zero();
                let spawn_pos = tf.translation + tf.forward() * (impact_dist - BACKWARD_DIST);
                resources.registry.spawn(
                    &item.0,
                    &mut commands,
                    Transform::from_translation(spawn_pos),
                );
                if let Ok((mut tf, mut spawner)) = particles.get_mut(effects.spawn_particles) {
                    tf.translation = spawn_pos;
                    spawner.reset();
                }
            }
        }
    }
}
