use bevy::prelude::*;

use super::movement::Velocity;

pub(crate) struct InterpolationPlugin;

impl Plugin for InterpolationPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            InterpolationSystemSet.before(TransformSystem::TransformPropagate),
        )
        .add_systems(
            PostUpdate,
            interpolate_transform.in_set(InterpolationSystemSet),
        )
        .add_systems(FixedFirst, (snap_transform_interpolation).chain())
        .add_systems(FixedLast, set_transform_interpolation_end)
        .register_type::<TransformInterpolationState>();

        //create state when entity spawned
        app.observe(
            |trigger: Trigger<OnAdd, Transform>,
             query: Query<&Transform, (With<Velocity>, Without<NoTransformInterpolation>)>,
             mut commands: Commands| {
                if let Ok(tf) = query.get(trigger.entity()) {
                    if let Some(mut ec) = commands.get_entity(trigger.entity()) {
                        ec.try_insert(TransformInterpolationState {
                            start_translation: tf.translation,
                            end_translation: tf.translation,
                        });
                    }
                }
            },
        );

        // create state when entity loses "NoTransformInterpolation" components
        // (idk why you would remove this after spawning, but why not support it)
        app.observe(
            |trigger: Trigger<OnRemove, NoTransformInterpolation>,
             query: Query<&Transform, (With<Velocity>, Without<NoTransformInterpolation>)>,
             mut commands: Commands| {
                if let Ok(tf) = query.get(trigger.entity()) {
                    if let Some(mut ec) = commands.get_entity(trigger.entity()) {
                        ec.try_insert(TransformInterpolationState {
                            start_translation: tf.translation,
                            end_translation: tf.translation,
                        });
                    }
                }
            },
        );
    }
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct InterpolationSystemSet;

#[derive(Component)]
pub struct NoTransformInterpolation;

//todo - make this more efficient. I feel like this can be cut down somehow
// (maybe being clever with lerping)
#[derive(Component, Reflect)]
pub(crate) struct TransformInterpolationState {
    pub(crate) start_translation: Vec3,
    pub(crate) end_translation: Vec3,
}

fn snap_transform_interpolation(
    mut set: ParamSet<(
        // first update, may not have interpolationstate set properly
        Query<
            (&Transform, &mut TransformInterpolationState),
            (
                Without<NoTransformInterpolation>,
                Added<TransformInterpolationState>,
            ),
        >,
        // normal path
        Query<
            (&mut Transform, &mut TransformInterpolationState),
            Without<NoTransformInterpolation>,
        >,
    )>,
) {
    for (tf, mut state) in &mut set.p0() {
        // first update, may not have interpolationstate set properly, so don't snap to state.end
        state.end_translation = tf.translation;
        state.start_translation = tf.translation;
    }
    for (mut tf, mut state) in &mut set.p1() {
        tf.translation = state.end_translation;
        state.start_translation = tf.translation;
    }
}

fn interpolate_transform(
    mut query: Query<
        (&mut Transform, &TransformInterpolationState),
        Without<NoTransformInterpolation>,
    >,
    time: Res<Time<Fixed>>,
) {
    let delta = time.overstep_fraction();
    for (
        mut tf,
        TransformInterpolationState {
            start_translation,
            end_translation,
        },
    ) in query.iter_mut()
    {
        // right now, physics doesn't do rotation/scale, so I think we can have the perf and only do translation
        tf.translation = start_translation.lerp(*end_translation, delta);
        // tf.rotation = start.rotation.slerp(end.rotation, delta);
        // tf.scale = start.scale.lerp(end.scale, delta);
    }
}

fn set_transform_interpolation_end(
    mut query: Query<
        (&Transform, &mut TransformInterpolationState),
        Without<NoTransformInterpolation>,
    >,
) {
    for (tf, mut state) in query.iter_mut() {
        state.end_translation = tf.translation;
    }
}
