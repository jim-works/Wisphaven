use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;
use bevy_atmosphere::prelude::*;
//https://github.com/JonahPlusPlus/bevy_atmosphere/blob/2ef39e2511fcb637ef83e507b468c4f5186c6913/examples/cycle.rs

#[derive(Component)]
struct Sun;

#[derive(Resource)]
struct CycleTimer(Timer);

pub struct AtmospherePlugin;

#[derive(Resource, Debug)]
pub struct Calendar {
    pub day_length: Duration,
    pub night_length: Duration,
    pub time: GameTime,
}

#[derive(Default, PartialEq, Eq, Clone, Copy, Debug)]
pub struct GameTime {
    pub day: u64,
    pub time: Duration,
}

impl GameTime {
    pub fn new(day: u64, time: Duration) -> Self {
        Self { day, time }
    }
}

impl PartialOrd for GameTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.day.partial_cmp(&other.day) {
            Some(core::cmp::Ordering::Equal) => self.time.partial_cmp(&other.time),
            ord => ord,
        }
    }
}

impl Calendar {
    pub fn total_day_length(&self) -> Duration {
        self.day_length + self.night_length
    }
    pub fn in_day(&self) -> bool {
        self.time.time < self.day_length
    }
    pub fn in_night(&self) -> bool {
        !self.in_day()
    }
    //maps day_elapsed to [0,1] where [0, 0.5) is day and [0.5, 1] is night
    pub fn get_sun_progress(&self) -> f32 {
        let total_length = self.total_day_length().as_secs_f32();
        let day_prop = self.day_length.as_secs_f32() / total_length;
        let night_prop = self.night_length.as_secs_f32() / total_length;
        let progress = self.time.time.as_secs_f32() / total_length;
        return if self.in_day() {
            0.5 * progress / day_prop
        } else {
            0.5 + 0.5 * (progress - day_prop) / night_prop
        };
    }

    //todo:
    //cannot handle amounts greater than 1 day or night
    pub fn advance(
        &mut self,
        amount: Duration,
        day_writer: &mut EventWriter<DayStartedEvent>,
        night_writer: &mut EventWriter<NightStartedEvent>,
    ) {
        if amount >= self.day_length.min(self.night_length) {
            //cannot handle amounts greater than 1 day
            return;
        }
        let was_in_day = self.in_day();
        self.time.time += amount;
        if was_in_day && !self.in_day() {
            night_writer.send(NightStartedEvent);
        } else if self.time.time >= self.total_day_length() {
            self.time.day += 1;
            self.time.time -= self.total_day_length();
            day_writer.send(DayStartedEvent);
        }
    }

    //scaled time, not affected by CalendarSpeed
    pub fn time_until(&self, time: GameTime) -> Duration {
        self.total_day_length() * (time.day - self.time.day) as u32 - (time.time - self.time.time)
    }
}

#[derive(Resource)]
//may overshoot if laggy
pub struct CalendarSpeed {
    pub fast_forward_timescale: f32,
    pub target: GameTime,
}

#[derive(Event)]
pub struct SkipDays {
    days: u64,
    end_time: Duration,
}

#[derive(Event)]
pub struct DayStartedEvent;
#[derive(Event)]
pub struct NightStartedEvent;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_atmosphere::plugin::AtmospherePlugin)
            .insert_resource(AtmosphereModel::new(Nishita {
                rayleigh_scale_height: 12e3,
                mie_scale_height: 1.8e3,
                ..default()
            }))
            .add_systems(Startup, setup_environment)
            .add_systems(PreUpdate, (update_sun_position, update_calendar))
            .insert_resource(CycleTimer(Timer::new(
                bevy::utils::Duration::from_millis(100),
                TimerMode::Repeating,
            )))
            .add_event::<SkipDays>()
            .add_event::<DayStartedEvent>()
            .add_event::<NightStartedEvent>()
            .insert_resource(CalendarSpeed {
                fast_forward_timescale: 50.0,
                target: GameTime::default(),
            })
            .insert_resource(Calendar {
                day_length: Duration::from_secs(5),
                night_length: Duration::from_secs(5),
                time: GameTime::default(),
            });
    }
}

fn update_sun_position(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut timer: ResMut<CycleTimer>,
    calendar: Res<Calendar>,
    time: Res<Time>,
) {
    let _my_span = info_span!("daylight_cycle", name = "daylight_cycle").entered();
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t = calendar.get_sun_progress() * 2.0 * PI;

        if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
            let sun_rot = Quat::from_rotation_x(-t);
            light_trans.rotation = sun_rot;
            //rotate backward vector (since light points away from the sun)
            atmosphere.sun_position = sun_rot * Vec3::new(0.0, 0.0, 1.0);
            directional.illuminance = t.sin().max(0.0).powf(2.0) * 100000.0;
        }
    }
}

fn update_calendar(
    time: Res<Time>,
    mut calendar: ResMut<Calendar>,
    speed: Res<CalendarSpeed>,
    mut day_writer: EventWriter<DayStartedEvent>,
    mut night_writer: EventWriter<NightStartedEvent>,
) {
    let inc = if calendar.time < speed.target {
        calendar
            .time_until(speed.target)
            .min(time.delta().mul_f32(speed.fast_forward_timescale))
    } else {
        time.delta()
    };
    calendar.advance(inc, &mut day_writer, &mut night_writer);
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
