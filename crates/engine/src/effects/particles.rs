use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::{actors::DamageTakenEvent, effects::EFFECT_GRAVITY, util::bevy_utils::TimedDespawner};

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_damage_particles)
            .add_systems(
                Update,
                (
                    spawn_damage_particles,
                    //todo - create system set and move to post update to avoid 1 frame lag when spawning particles
                    spawn_particles_on_attack,
                ),
            )
            .add_event::<SpawnDamageParticles>();
    }
}

#[derive(Event)]
pub struct SpawnDamageParticles {
    pub origin: Vec3,
}

#[derive(Resource)]
struct DamageParticles {
    effect: Handle<EffectAsset>,
}

const MAX_DAMAGE_PARTICLE_LIFETIME: f32 = 1.2;

fn setup_damage_particles(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    const MAX_DAMAGE_PARTICLES: u32 = 512;
    const DAMAGE_PARTICLES_PER_HIT: f32 = 16.0;

    let writer = ExprWriter::new();

    let emit_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(1.).expr(),
        dimension: ShapeDimension::Volume,
    };

    let emit_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(2.).expr(),
    };

    let accel = AccelModifier::new(writer.lit(EFFECT_GRAVITY).expr());

    let emit_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer
            .lit(0.8)
            .uniform(writer.lit(MAX_DAMAGE_PARTICLE_LIFETIME))
            .expr(),
    );

    let size = SizeOverLifetimeModifier {
        gradient: Gradient::from_keys([
            (0.0, Vec2::splat(0.2)),
            (0.8, Vec2::splat(0.2)),
            (1.0, Vec2::ZERO),
        ]),
        screen_space_size: false,
    };

    let orientation = OrientModifier {
        mode: OrientMode::FaceCameraPosition,
        rotation: None,
    };

    let color = ColorOverLifetimeModifier {
        gradient: Gradient::constant(Srgba::hex("8c1529").unwrap().to_vec4()),
    };

    let effect = effects.add(
        EffectAsset::new(
            vec![MAX_DAMAGE_PARTICLES],
            Spawner::once(DAMAGE_PARTICLES_PER_HIT.into(), true),
            writer.finish(),
        )
        .with_name("damage_particles")
        .init(emit_pos)
        .init(emit_vel)
        .init(emit_lifetime)
        .update(accel)
        .render(size)
        .render(orientation)
        .render(color),
    );
    commands.insert_resource(DamageParticles { effect })
}

fn spawn_damage_particles(
    mut commands: Commands,
    particles: Res<DamageParticles>,
    mut reader: EventReader<SpawnDamageParticles>,
) {
    for event in reader.read() {
        //can set scale/color later by setting custom properties on the particle effet
        commands.spawn((
            Name::new("damage_particle"),
            ParticleEffectBundle {
                effect: ParticleEffect::new(particles.effect.clone()),
                transform: Transform::from_translation(event.origin),
                ..default()
            },
            TimedDespawner(Timer::from_seconds(
                MAX_DAMAGE_PARTICLE_LIFETIME,
                TimerMode::Once,
            )),
        ));
    }
}

fn spawn_particles_on_attack(
    mut particles_writer: EventWriter<SpawnDamageParticles>,
    mut attack_reader: EventReader<DamageTakenEvent>,
) {
    for event in attack_reader.read() {
        particles_writer.send(SpawnDamageParticles {
            origin: event.hit_location,
        });
    }
}
