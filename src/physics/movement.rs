use bevy::{prelude::*, transform::TransformSystem};
use serde::{Serialize, Deserialize};

use crate::{physics::TPS, util::{iterators::AxisMap, project_onto_plane}};

use super::{
    collision::{CollidingBlocks, Friction, IgnoreTerrainCollision},
    PhysicsSystemSet,
};

//local space, without local rotation
#[derive(Component, Default, Deref, DerefMut, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Velocity(pub Vec3);

//local space, without local rotation
//optional - acceleration not due to gravity
#[derive(Component, Default, Deref, DerefMut, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Acceleration(pub Vec3);

//local space, without local rotation
#[derive(Resource, Deref, DerefMut, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Gravity(pub Vec3);

//children of a parent should not typically have separate GravityMults unless the parent will not rotate
//gravity is taken in local space without local rotation, so parent's rotation will affect the gravity direction
#[derive(Component, Deref, DerefMut, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GravityMult(pub f32);

impl Default for GravityMult {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Component, Deref, DerefMut, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Drag(pub f32);

impl Default for Drag {
    fn default() -> Self {
        Self(0.025)
    }
}

#[derive(Component, Deref, DerefMut, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Mass(pub f32);

impl Default for Mass {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Mass {
    pub fn add_force(self, f: Vec3, accel: &mut Acceleration) {
        accel.0 += f / self.0
    }
    pub fn add_impulse(self, i: Vec3, vel: &mut Velocity) {
        vel.0 += i / self.0
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
        app.insert_resource(Gravity(Vec3::new(0.0, -0.01, 0.0)))
            .add_systems(
                FixedUpdate,
                snap_tf_interpolation.in_set(PhysicsSystemSet::ResetInterpolation),
            )
            .add_systems(
                FixedUpdate,
                translate.in_set(PhysicsSystemSet::UpdatePosition),
            )
            .add_systems(
                FixedUpdate,
                (update_friction, update_drag, update_derivatives)
                    .chain()
                    .in_set(PhysicsSystemSet::UpdateDerivatives),
            )
            .add_systems(
                FixedUpdate,
                set_tf_interpolation_target.after(PhysicsSystemSet::UpdateDerivatives),
            )
            .add_systems(
                PostUpdate,
                interpolate_tf_translation.before(TransformSystem::TransformPropagate),
            );
    }
}

fn update_derivatives(
    mut query: Query<(&mut Velocity, &mut Acceleration, Option<&GravityMult>)>,
    gravity: Res<Gravity>,
) {
    const EPSILON: f32 = 0.0001;
    for (mut v, mut a, opt_g) in query.iter_mut() {
        //min move speed to alleviate imprecision/jittering
        v.0 = v.0.axis_map(|e| if e.abs() < EPSILON { 0.0 } else { e });
        
        v.0 += a.0;
        //reset acceleration
        a.0 = opt_g.map(|g| g.0).unwrap_or(0.0) * gravity.0;
    }
}

fn update_friction(mut query: Query<(&mut Velocity, &Acceleration, &CollidingBlocks, &Friction)>, block_query: Query<&Friction>) {
    const EPSILON: f32 = 0.0001;
    for (mut v, a, blocks, f) in query.iter_mut() {
        blocks.for_each_dir(|dir, blocks| {
            if blocks.is_empty() {
                return; //nothing to friction with
            }
            //get avg friction of all collided blocks
            let mut sum_fric_coeff = 0.0;
            for (_, e, _) in blocks.iter() {
                sum_fric_coeff += block_query.get(*e).ok().and_then(|f| Some(f.0)).unwrap_or_default();
            }
            //total friction is block avg friction combined with entity's friction
            let f = ((sum_fric_coeff / blocks.len() as f32) + f.0) / 2.0;
            //calculate friction vector in the plane of the block
            let normal = dir.opposite().to_vec3();
            let planar_v = project_onto_plane(v.0, normal).normalize_or_zero();
            let fric_mag = -a.0.dot(normal)*f;
            if fric_mag <= 0.0 {
                //make sure friction slows us down
                if v.0.length_squared() < EPSILON {
                    //prevent sliding forever super slow due to imprecision
                    v.0 = Vec3::ZERO;
                } else {
                    v.0 += planar_v*fric_mag;
                }
            }
        })
    }
}

fn update_drag(mut query: Query<(&mut Velocity, &Drag)>) {
    for (mut v, d) in query.iter_mut() {
        v.0 *= 1.0-d.0;
    }
}

//simpler (and faster) to extract this out
fn translate(mut query: Query<(&mut Transform, &Velocity), With<IgnoreTerrainCollision>>) {
    for (mut tf, v) in query.iter_mut() {
        tf.translation += v.0;
    }
}

fn set_tf_interpolation_target(
    mut query: Query<(&mut Transform, &mut InterpolatedAttribute<Transform>)>,
) {
    for (mut tf, mut interpolator) in query.iter_mut() {
        interpolator.old = interpolator.target;
        interpolator.target = *tf;
        tf.translation = interpolator.old.translation;
    }
}

fn snap_tf_interpolation(mut query: Query<(&mut Transform, &InterpolatedAttribute<Transform>)>) {
    for (mut tf, interpolator) in query.iter_mut() {
        tf.translation = interpolator.target.translation;
    }
}

fn interpolate_tf_translation(
    mut query: Query<(&mut Transform, &InterpolatedAttribute<Transform>)>,
    time: Res<Time>,
) {
    //lerp speed needs to be slower if tick rate is slower
    //passes eye test if TPS > 20 ish, which is all we care about fr fr
    //probably should improve for lower TPS
    let lerp_time = (time.delta_seconds() * TPS as f32).min(1.0);
    for (mut tf, interpolator) in query.iter_mut() {
        tf.translation = tf
            .translation
            .lerp(interpolator.target.translation, lerp_time);
    }
}
