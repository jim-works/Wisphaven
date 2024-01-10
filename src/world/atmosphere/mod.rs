use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;

#[derive(Component)]
struct Sun;

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
        Some(self.cmp(other))
    }
}

impl Ord for GameTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.day.cmp(&other.day) {
            core::cmp::Ordering::Equal => self.time.cmp(&other.time),
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
        if self.in_day() {
            0.5 * progress / day_prop
        } else {
            0.5 + 0.5 * (progress - day_prop) / night_prop
        }
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
        (self.total_day_length() * time.day.saturating_sub(self.time.day) as u32)
            + time.time.saturating_sub(self.time.time)
    }

    pub fn next_night(&self) -> GameTime {
        if self.time.time >= self.day_length {
            GameTime::new(self.time.day + 1, self.day_length)
        } else {
            GameTime::new(self.time.day, self.day_length)
        }
    }

    pub fn next_day(&self) -> GameTime {
        GameTime::new(self.time.day + 1, Duration::ZERO)
    }
}

#[derive(Resource)]
//may overshoot if laggy
struct CalendarSpeed {
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
#[derive(Event)]
pub struct SpeedupCalendarEvent(pub GameTime);

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_environment)
            .add_systems(
                PreUpdate,
                (speedup_time, update_calendar, update_sun_position).chain(),
            )
            .add_event::<SkipDays>()
            .add_event::<DayStartedEvent>()
            .add_event::<NightStartedEvent>()
            .add_event::<SpeedupCalendarEvent>()
            .insert_resource(CalendarSpeed {
                fast_forward_timescale: 50.0,
                target: GameTime::default(),
            })
            .insert_resource(Calendar {
                day_length: Duration::from_secs(600),
                night_length: Duration::from_secs(300),
                time: GameTime::new(0, Duration::from_secs(100)),
            });
    }
}

fn update_sun_position(
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    calendar: Res<Calendar>,
    time: Res<Time>,
) {
    let _my_span = info_span!("daylight_cycle", name = "daylight_cycle").entered();

    let t = calendar.get_sun_progress() * 2.0 * PI;

    if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
        let sun_rot = Quat::from_rotation_x(-t);
        light_trans.rotation = sun_rot;
        directional.illuminance = t.sin().max(0.0).powf(2.0) * 100000.0;
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

fn speedup_time(mut reader: EventReader<SpeedupCalendarEvent>, mut speed: ResMut<CalendarSpeed>) {
    for SpeedupCalendarEvent(time) in reader.read() {
        info!(
            "setting target time to {:?} (submitted {:?}",
            speed.target.max(*time),
            time
        );
        speed.target = speed.target.max(*time);
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
        Name::new("Sun"),
    ));
}
