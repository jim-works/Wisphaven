use crate::world::LevelSystemSet;
use bevy::prelude::*;
use bevy_rapier3d::{prelude::*, rapier::prelude::CollisionEvent};

use super::Damage;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, test_projectile_hit.in_set(LevelSystemSet::Main));
    }
}

pub struct ProjectileHit<'a> {
    hit: Option<Entity>,
    projectile: Entity,
    commands: &'a mut Commands<'a, 'a>,
}

#[derive(Component)]
pub struct Projectile {
    pub damage: Damage,
    pub on_hit: Box<dyn Fn(ProjectileHit) + Send + Sync>,
}

#[derive(Bundle)]
pub struct ProjectileBundle {
    pub projectile: Projectile,
    pub active_events: ActiveEvents,
}

impl ProjectileBundle {
    pub fn new(projectile: Projectile) -> Self {
        Self {
            projectile,
            active_events: ActiveEvents::COLLISION_EVENTS,
        }
    }
}

fn test_projectile_hit(
    query: Query<(Entity, &Projectile, &GlobalTransform, &Collider)>,
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>
) {
    const EPSILON: f32 = 1e-3;
    const DETECT_DIST: f32 = 0.05;
    for event in collision_events.iter() {
        match event {
            
        }
    }
    for (entity, mut grounded, tf, col) in query.iter() {
        //check on ground
        // let groups = QueryFilter::default().exclude_collider(entity);
        // if let Some((hit_entity, _)) = ctx.cast_shape(
        //     tf.translation(),
        //     Quat::IDENTITY,
        //     Vec3::new(0.0, DETECT_DIST, 0.0),
        //     col,
        //     1.0,
        //     groups,
        // ) {

        // }
    }
}
