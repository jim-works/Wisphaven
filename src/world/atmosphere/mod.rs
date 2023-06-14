use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_atmosphere::prelude::*;

//https://github.com/JonahPlusPlus/bevy_atmosphere/blob/2ef39e2511fcb637ef83e507b468c4f5186c6913/examples/cycle.rs

#[derive(Component)]
struct Sun;

#[derive(Resource)]
struct CycleTimer(Timer);

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup_environment)
            .add_system(daylight_cycle)
            .insert_resource(AtmosphereModel::default())
            .insert_resource(CycleTimer(Timer::new(
                bevy::utils::Duration::from_millis(100),
                TimerMode::Repeating,
            )));
    }
}

fn daylight_cycle(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut timer: ResMut<CycleTimer>,
    time: Res<Time>,
) {
    const DAY_CYCLE_SECONDS: f32 = 60.0*10.0;
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t = (time.elapsed_seconds_wrapped() as f32 / DAY_CYCLE_SECONDS)*2.0*PI;
        atmosphere.sun_position = Vec3::new(0.0, t.sin(), t.cos());

        if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
            light_trans.rotation = Quat::from_rotation_x(-t);
            directional.illuminance = t.sin().max(0.0).powf(2.0) * 100000.0;
        }
    }
}

fn setup_environment(mut commands: Commands) {
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        Sun, // Marks the light as Sun
    ));
}
