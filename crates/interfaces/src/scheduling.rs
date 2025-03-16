use bevy::prelude::*;

pub(crate) struct SchedulingPlugin;

impl Plugin for SchedulingPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .enable_state_scoped_entities::<GameState>()
            .add_sub_state::<LevelLoadState>()
            .enable_state_scoped_entities::<LevelLoadState>()
            .init_state::<GameLoadState>()
            .enable_state_scoped_entities::<GameLoadState>()
            .init_state::<NetworkType>()
            .enable_state_scoped_entities::<NetworkType>()
            .init_state::<ClientState>()
            .enable_state_scoped_entities::<ClientState>()
            .init_state::<ServerState>()
            .enable_state_scoped_entities::<ServerState>()
            .init_state::<DebugUIState>()
            .enable_state_scoped_entities::<DebugUIState>()
            .init_state::<DebugUIDetailState>()
            .enable_state_scoped_entities::<DebugUIDetailState>()
            .configure_sets(
                PostUpdate,
                LevelSystemSet::PostUpdate
                    .run_if(in_state(LevelLoadState::Loaded))
                    .run_if(in_state(GameState::Game)),
            )
            .configure_sets(
                Update,
                LevelSystemSet::AfterLoadingAndMain
                    .run_if(in_state(LevelLoadState::Loading).or(in_state(LevelLoadState::Loaded)))
                    .run_if(in_state(GameState::Game))
                    .after(LevelSystemSet::LoadingAndMain)
                    .after(LevelSystemSet::Main),
            )
            .configure_sets(
                Update,
                LevelSystemSet::Main
                    .run_if(in_state(LevelLoadState::Loaded))
                    .run_if(in_state(GameState::Game))
                    .after(UtilSystemSet),
            )
            .configure_sets(
                Update,
                LevelSystemSet::Despawn
                    .after(LevelSystemSet::Main)
                    .after(LevelSystemSet::LoadingAndMain)
                    .after(LevelSystemSet::AfterLoadingAndMain),
            )
            .configure_sets(
                Update,
                LevelSystemSet::LoadingAndMain
                    .run_if(in_state(LevelLoadState::Loading).or(in_state(LevelLoadState::Loaded)))
                    .run_if(in_state(GameState::Game))
                    .after(UtilSystemSet),
            )
            .configure_sets(
                FixedUpdate,
                (
                    LevelSystemSet::PreTick
                        .before(PhysicsSystemSet::Main)
                        .after(UtilSystemSet),
                    LevelSystemSet::Tick.in_set(PhysicsSystemSet::Main),
                    LevelSystemSet::PostTick.after(PhysicsSystemSet::UpdateDerivatives),
                )
                    .chain()
                    .run_if(in_state(LevelLoadState::Loaded))
                    .run_if(in_state(GameState::Game)),
            )
            .add_systems(
                Update,
                apply_deferred
                    .after(LevelSystemSet::Main)
                    .after(LevelSystemSet::LoadingAndMain)
                    .before(LevelSystemSet::AfterLoadingAndMain),
            )
            .configure_sets(
                FixedUpdate,
                (
                    PhysicsSystemSet::Main.after(UtilSystemSet),
                    PhysicsSystemSet::ProcessRaycasts,
                    PhysicsSystemSet::UpdatePosition,
                    PhysicsSystemSet::UpdateDerivatives,
                )
                    .chain(),
            )
            .configure_sets(
                FixedUpdate,
                PhysicsLevelSet::Main
                    .in_set(PhysicsSystemSet::Main)
                    .run_if(in_state(LevelLoadState::Loaded)),
            )
            .configure_sets(
                FixedUpdate,
                TransformSystem::TransformPropagate.after(PhysicsSystemSet::UpdatePosition),
            )
            .configure_sets(FixedUpdate, UtilSystemSet)
            .configure_sets(Update, UtilSystemSet)
            .configure_sets(
                Update,
                (
                    ItemSystemSet::Usage.in_set(LevelSystemSet::Main),
                    ItemSystemSet::UsageProcessing.in_set(LevelSystemSet::Main),
                    ItemSystemSet::DropPickup.in_set(LevelSystemSet::Main),
                    ItemSystemSet::DropPickupProcessing.in_set(LevelSystemSet::Main),
                )
                    .chain(),
            );
    }
}

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameState {
    #[default]
    Setup,
    Menu,
    Game,
    GameOver,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum LevelSystemSet {
    //systems in main should not despawn any entities, and don't have to worry about entity despawning. only runs in LevelLoadState::Loaded
    Main,
    //all the despawning happens in the despawn set. only runs in LevelLoadState::Loaded
    Despawn,
    //Post-update, runs after both main and despawn, in LevelLoadState::Loaded
    PostUpdate,
    //like main, but also runs in only runs in LevelLoadState::Loading
    LoadingAndMain,
    //Update, runs after main/loading in main, in LevelLoadState::Loaded and Loading
    //system buffers from main and loading and main applied beforehand
    AfterLoadingAndMain,
    //fixedupdate
    PreTick,
    Tick,
    PostTick,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, SubStates, Default)]
#[source(GameState = GameState::Game)]
pub enum LevelLoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded,
}

//run in fixed update
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PhysicsSystemSet {
    Main, //all user code should run here
    ProcessRaycasts,
    UpdatePosition,
    UpdateDerivatives,
}

//run in fixed update
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PhysicsLevelSet {
    Main,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct UtilSystemSet;

#[derive(States, Default, Debug, Hash, Eq, PartialEq, Clone)]
pub enum GameLoadState {
    #[default]
    Preloading,
    LoadingAssets,
    Done,
}

#[derive(States, Hash, Eq, PartialEq, Copy, Clone, Debug, Default)]
pub enum NetworkType {
    #[default]
    Inactive,
    Server,
    Client,
    Host,
}

impl NetworkType {
    pub fn is_server(self) -> bool {
        matches!(self, NetworkType::Server | NetworkType::Host)
    }
    pub fn is_client(self) -> bool {
        matches!(self, NetworkType::Client | NetworkType::Host)
    }
    pub fn to_network_mode(self) -> lightyear::prelude::Mode {
        match self {
            NetworkType::Host => lightyear::prelude::Mode::HostServer,
            _ => lightyear::prelude::Mode::Separate,
        }
    }
}

#[derive(States, Default, Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub enum ClientState {
    #[default]
    NotStarted,
    Started,
    //recieved initialization message from server
    Ready,
}

#[derive(States, Default, Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub enum ServerState {
    #[default]
    NotStarted,
    Active,
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub enum DebugUIState {
    #[default]
    Hidden,
    Shown,
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub enum DebugUIDetailState {
    #[default]
    Minimal,
    Most,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum ItemSystemSet {
    Usage,
    UsageProcessing,
    DropPickup,
    DropPickupProcessing,
}
