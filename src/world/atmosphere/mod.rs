use std::f32::consts::PI;

use bevy::prelude::*;

//https://github.com/JonahPlusPlus/bevy_atmosphere/blob/2ef39e2511fcb637ef83e507b468c4f5186c6913/examples/cycle.rs

#[derive(Component)]
struct Sun;

#[derive(Resource)]
struct CycleTimer(Timer);

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_environment)
            .add_systems(Update, daylight_cycle)
            .insert_resource(CycleTimer(Timer::new(
                bevy::utils::Duration::from_millis(100),
                TimerMode::Repeating,
            )));
    }
}

fn daylight_cycle(
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut timer: ResMut<CycleTimer>,
    time: Res<Time>,
) {
    let _my_span = info_span!("daylight_cycle", name = "daylight_cycle").entered();
    const DAY_CYCLE_SECONDS: f32 = 60.0*10.0;
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t = ((time.elapsed_seconds_wrapped()+60.0) / DAY_CYCLE_SECONDS)*2.0*PI;

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
