use bevy::prelude::*;

use super::{PhysicsSystemSet, TICK_SCALE};

//local space, without local rotation
#[derive(Component, Default, Deref, DerefMut, PartialEq, Clone, Copy)]
pub struct Velocity(pub Vec3);

//local space, without local rotation
//optional - acceleration not due to gravity
#[derive(Component, Default, Deref, DerefMut, PartialEq, Clone, Copy)]
pub struct Acceleration(pub Vec3);

//local space, without local rotation
#[derive(Resource, Deref, DerefMut, PartialEq, Clone, Copy)]
pub struct Gravity(pub Vec3);

//children of a parent should not typically have separate GravityMults unless the parent will not rotate
//gravity is taken in local space without local rotation, so parent's rotation will affect the gravity direction
#[derive(Component, Deref, DerefMut, PartialEq, Clone, Copy)]
pub struct GravityMult(pub f32);

impl Default for GravityMult {
    fn default() -> Self {
        Self(1.0)
    }
}

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity(Vec3::new(0.0, -0.005, 0.0)))
            .add_systems(
                FixedUpdate,
                (update_kinematics_no_acceleration, update_kinematics_acceleration).in_set(PhysicsSystemSet::UpdatePositionVelocity),
            );
    }
}

//simpler (and faster) to extract this out
fn update_kinematics_no_acceleration(
    mut query: Query<(&mut Transform, &Velocity), Without<Acceleration>>,
) {
    for (mut tf, v) in query.iter_mut() {
        tf.translation += v.0 * TICK_SCALE as f32;
    }
}

fn update_kinematics_acceleration(
    mut query: Query<(
        &mut Transform,
        &mut Velocity,
        &mut Acceleration,
        Option<&GravityMult>,
    )>,
    gravity: Res<Gravity>,
) {
    for (mut tf, mut v, mut a, opt_g) in query.iter_mut() {
        //adding half acceleration for proper integration
        tf.translation += v.0 * TICK_SCALE as f32 + 0.5 * a.0 * (TICK_SCALE * TICK_SCALE) as f32;
        v.0 += a.0 * TICK_SCALE as f32;
        //reset acceleration
        a.0 = opt_g.map(|g| g.0).unwrap_or(0.0) * gravity.0;
    }
}
