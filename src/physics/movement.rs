use bevy::{prelude::*, transform::TransformSystem};

use crate::physics::TPS;

use super::{collision::IgnoreTerrainCollision, PhysicsSystemSet};

//local space, without local rotation
#[derive(Component, Default, Deref, DerefMut, PartialEq, Clone, Copy, Debug)]
pub struct Velocity(pub Vec3);

//local space, without local rotation
//optional - acceleration not due to gravity
#[derive(Component, Default, Deref, DerefMut, PartialEq, Clone, Copy, Debug)]
pub struct Acceleration(pub Vec3);

//local space, without local rotation
#[derive(Resource, Deref, DerefMut, PartialEq, Clone, Copy, Debug)]
pub struct Gravity(pub Vec3);

//children of a parent should not typically have separate GravityMults unless the parent will not rotate
//gravity is taken in local space without local rotation, so parent's rotation will affect the gravity direction
#[derive(Component, Deref, DerefMut, PartialEq, Clone, Copy, Debug)]
pub struct GravityMult(pub f32);

impl Default for GravityMult {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Component)]
pub struct InterpolatedAttribute<T: Component> {
    pub old: T,
    pub target: T,
}

impl<T: Component + Clone> From<T> for InterpolatedAttribute<T> {
    fn from(value: T) -> Self {
        Self {
            old: value.clone(),
            target: value,
        }
    }
}

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity(Vec3::new(0.0, -0.005, 0.0)))
            // .add_systems(
            //     FixedUpdate,
            //     snap_tf_interpolation.in_set(PhysicsSystemSet::ResetInterpolation),
            // )
            // .add_systems(
            //     FixedUpdate,
            //     (translate_no_acceleration, translate_acceleration)
            //         .in_set(PhysicsSystemSet::UpdatePosition),
            // )
            .add_systems(
                FixedUpdate,
                update_derivatives.in_set(PhysicsSystemSet::UpdateDerivatives),
            )
            // .add_systems(
            //     FixedUpdate,
            //     set_tf_interpolation_target.after(PhysicsSystemSet::UpdateDerivatives),
            // )
            // .add_systems(
            //     PostUpdate,
            //     interpolate_tf_translation.before(TransformSystem::TransformPropagate),
            // );
            ;
    }
}

fn update_derivatives(
    mut query: Query<(&mut Velocity, &mut Acceleration, Option<&GravityMult>)>,
    gravity: Res<Gravity>,
) {
    for (mut v, mut a, opt_g) in query.iter_mut() {
        v.0 += a.0;
        //reset acceleration
        a.0 = opt_g.map(|g| g.0).unwrap_or(0.0) * gravity.0;
    }
}

//simpler (and faster) to extract this out
fn translate_no_acceleration(
    mut query: Query<
        (&mut Transform, &Velocity),
        (Without<Acceleration>, With<IgnoreTerrainCollision>),
    >,
) {
    for (mut tf, v) in query.iter_mut() {
        tf.translation += v.0;
    }
}

fn translate_acceleration(
    mut query: Query<(&mut Transform, &Velocity, &Acceleration), With<IgnoreTerrainCollision>>,
) {
    for (mut tf, v, a) in query.iter_mut() {
        //adding half acceleration for proper integration
        tf.translation += v.0 + 0.5 * a.0;
    }
}

fn set_tf_interpolation_target(
    mut query: Query<(&mut Transform, &mut InterpolatedAttribute<Transform>)>,
) {
    for (mut tf, mut interpolator) in query.iter_mut() {
        interpolator.old = interpolator.target;
        interpolator.target = *tf;
        *tf = interpolator.old;
    }
}

fn snap_tf_interpolation(mut query: Query<(&mut Transform, &InterpolatedAttribute<Transform>)>) {
    for (mut tf, interpolator) in query.iter_mut() {
        *tf = interpolator.target;
    }
}

fn interpolate_tf_translation(
    mut query: Query<(&mut Transform, &InterpolatedAttribute<Transform>)>,
    time: Res<Time>,
) {
    //lerp speed needs to be slower if tick rate is slower
    //passes eye test, which is all we care about fr fr
    let lerp_time = (time.delta_seconds() * TPS as f32).min(1.0);
    for (mut tf, interpolator) in query.iter_mut() {
        tf.translation = tf
            .translation
            .lerp(interpolator.target.translation, lerp_time);
    }
}
