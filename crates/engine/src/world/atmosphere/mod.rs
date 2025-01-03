use std::{f32::consts::PI, time::Duration};

use bevy::{
    asset::LoadState,
    core_pipeline::Skybox,
    prelude::*,
    render::render_resource::{TextureViewDescriptor, TextureViewDimension},
};

use crate::{actors::world_anchor::ActiveWorldAnchor, GameState};

#[derive(Component, Reflect)]
struct Sun {
    strength: f32,
}

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

impl std::fmt::Display for GameTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 40 seconds equals one in-game hour
        let total_seconds = self.time.as_secs_f32();
        let in_game_hours = total_seconds / 40.;
        write!(f, "{} days and {:.2} hours", self.day, in_game_hours)
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
            info!("night started!")
        } else if self.time.time >= self.total_day_length() {
            self.time.day += 1;
            self.time.time -= self.total_day_length();
            day_writer.send(DayStartedEvent);
            info!("day started!");
        }
    }

    //todo:
    //cannot handle amounts greater than 1 day or night
    pub fn advance_eternal_night(
        &mut self,
        amount: Duration,
        night_writer: &mut EventWriter<NightStartedEvent>,
    ) {
        if amount >= self.day_length.min(self.night_length) {
            //cannot handle amounts greater than 1 day
            return;
        }
        let was_in_day = self.in_day();
        self.time.time += amount;
        if was_in_day && !self.in_day() {
            // account for transition to night if called during the day
            night_writer.send(NightStartedEvent);
        } else if self.time.time >= self.total_day_length() {
            // skip the day portion and go straight to night if originally in night
            self.time.day = self.next_night().day;
            self.time.time = (self.next_night().time + amount).min(self.total_day_length());
            night_writer.send(NightStartedEvent);
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

#[derive(Resource)]
pub struct LoadingSkyboxCubemap(pub Handle<Image>);
#[derive(Resource)]
pub struct SkyboxCubemap(pub Handle<Image>);

#[derive(Resource)]
pub struct Fog {
    base_color: Color,
    night_falloff: (f32, f32),
    day_falloff: (f32, f32),
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
        app.add_systems(Startup, (setup_environment, setup_calendar))
            .add_systems(OnEnter(GameState::Game), setup_calendar)
            .add_systems(OnEnter(GameState::Menu), spawn_sun)
            .add_systems(OnEnter(GameState::Game), spawn_sun)
            .add_systems(
                Update,
                load_skybox.run_if(resource_exists::<LoadingSkyboxCubemap>),
            )
            .add_systems(
                PreUpdate,
                (speedup_time, update_calendar, update_sky).chain(),
            )
            .add_event::<SkipDays>()
            .add_event::<DayStartedEvent>()
            .add_event::<NightStartedEvent>()
            .add_event::<SpeedupCalendarEvent>()
            .insert_resource(AmbientLight {
                brightness: 100.,
                ..default()
            })
            .insert_resource(Fog {
                base_color: Color::srgba(0.56, 0.824, 1.0, 1.0),
                day_falloff: (100.0, 200.0),
                night_falloff: (75.0, 150.0),
            })
            .register_type::<Sun>();
    }
}

fn load_skybox(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    loading_skybox: Res<LoadingSkyboxCubemap>,
) {
    if matches!(
        asset_server.load_state(&loading_skybox.0),
        LoadState::Loaded
    ) {
        let image = images.get_mut(&loading_skybox.0).unwrap();
        //transform png into cubemap
        if image.texture_descriptor.array_layer_count() == 1 {
            image.reinterpret_stacked_2d_as_array(image.height() / image.width());
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..default()
            });
        }
        commands.insert_resource(SkyboxCubemap(loading_skybox.0.clone()));
        commands.remove_resource::<LoadingSkyboxCubemap>();
    }
}

fn update_sky(
    mut sun_query: Query<(&mut Transform, &mut DirectionalLight, &Sun)>,
    mut skybox_query: Query<&mut Skybox>,
    mut fog_query: Query<&mut DistanceFog>,
    calendar: Res<Calendar>,
    fog_color: Res<Fog>,
) {
    let _my_span = info_span!("daylight_cycle", name = "daylight_cycle").entered();

    let t = calendar.get_sun_progress() * 2.0 * PI;

    if let Ok((mut light_trans, mut directional, sun)) = sun_query.get_single_mut() {
        let sun_rot = Quat::from_rotation_x(-t);
        light_trans.rotation = sun_rot;
        let sun_strength_factor = t.sin().max(0.0).powf(2.0);
        directional.illuminance = sun_strength_factor * sun.strength;
        if let Ok(mut skybox) = skybox_query.get_single_mut() {
            const SKYBOX_BRIGHTNESS_FACTOR: f32 = 0.15;
            skybox.brightness = sun_strength_factor * SKYBOX_BRIGHTNESS_FACTOR * sun.strength;
        }
        if let Ok(mut fog) = fog_query.get_single_mut() {
            fog.color = fog_color
                .base_color
                .mix(&Color::BLACK, 1.0 - sun_strength_factor);
            fog.falloff = FogFalloff::Linear {
                start: fog_color
                    .night_falloff
                    .0
                    .lerp(fog_color.day_falloff.0, sun_strength_factor),
                end: fog_color
                    .night_falloff
                    .1
                    .lerp(fog_color.day_falloff.1, sun_strength_factor),
            }
        }
    }
}

fn setup_calendar(mut commands: Commands) {
    commands.insert_resource(CalendarSpeed {
        fast_forward_timescale: 200.0,
        target: GameTime::default(),
    });
    // 40 seconds per in-game hour,so this is 24 hours split to 2/3 day and 1/3 night
    commands.insert_resource(Calendar {
        day_length: Duration::from_secs(640),
        night_length: Duration::from_secs(320),
        time: GameTime::new(0, Duration::from_secs(100)),
    });
}

fn update_calendar(
    time: Res<Time>,
    mut calendar: ResMut<Calendar>,
    speed: Res<CalendarSpeed>,
    mut day_writer: EventWriter<DayStartedEvent>,
    mut night_writer: EventWriter<NightStartedEvent>,
    world_anchor: Option<Res<ActiveWorldAnchor>>,
) {
    let inc = if calendar.time < speed.target {
        calendar
            .time_until(speed.target)
            .min(time.delta().mul_f32(speed.fast_forward_timescale))
    } else {
        time.delta()
    };
    match world_anchor {
        Some(_) => calendar.advance(inc, &mut day_writer, &mut night_writer),
        None => calendar.advance_eternal_night(inc, &mut night_writer),
    };
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

fn setup_environment(mut commands: Commands, asset_server: Res<AssetServer>) {
    let skybox = asset_server.load("textures/skybox.png");
    commands.insert_resource(LoadingSkyboxCubemap(skybox));
}

fn spawn_sun(mut commands: Commands, sun_query: Query<Entity, With<Sun>>) {
    for entity in sun_query.iter() {
        if let Some(mut ec) = commands.get_entity(entity) {
            ec.despawn();
        }
    }
    //want sun on main menu too, but want to reset it every time we transition game states
    #[allow(state_scoped_entities)]
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Sun { strength: 7500.0 }, // Marks the light as Sun
        Name::new("Sun"),
    ));
}
