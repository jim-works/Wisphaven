use bevy::prelude::*;

use super::{movement::Velocity, PhysicsSystemSet};

pub(crate) struct SpringPlugin;

impl Plugin for SpringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, update_springs.in_set(PhysicsSystemSet::Main));
    }
}

#[derive(Component)]
pub struct Spring {
    pub rest_length: f32,
    pub strength: f32,
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            rest_length: 1.,
            strength: 0.1,
        }
    }
}

#[derive(Component)]
pub struct SpringAttachment(pub Entity);

fn update_springs(
    mut set: ParamSet<(
        (
            Query<(Entity, &GlobalTransform, &Spring, &SpringAttachment)>,
            Query<&GlobalTransform>,
        ),
        Query<&mut Velocity>,
    )>,
) {
    let mut delta_vs = Vec::with_capacity(set.p0().0.iter().len());
    {
        let (springs, attachments) = set.p0();
        for (entity, gtf, spring, SpringAttachment(attachment)) in springs.iter() {
            if let Ok(attach_gtf) = attachments.get(*attachment) {
                let delta = attach_gtf.translation() - gtf.translation();
                let delta_mag = delta.length();
                let extension = delta_mag - spring.rest_length;

                if delta_mag > f32::EPSILON {
                    let delta_v = spring.strength * extension * delta / delta_mag;
                    info!(
                        "delta v {}, delta {}, extension {}",
                        delta_v, delta, extension
                    );
                    delta_vs.push((entity, delta_v));
                }
            }
        }
    }
    let mut v_query = set.p1();
    for (entity, dv) in delta_vs.into_iter() {
        if let Ok(mut v) = v_query.get_mut(entity) {
            v.0 += dv;
        }
    }
}
