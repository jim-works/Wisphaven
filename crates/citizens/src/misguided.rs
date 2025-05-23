use bevy::prelude::*;
use dialogue::{ActiveDialogue, Dialogues};
use engine::{
    actors::{ActorName, BuildActorRegistry, SpawnActorEvent},
    items::inventory::Inventory,
};
use interfaces::{components::Interactable, events::InteractedEvent, scheduling::LevelSystemSet};
use serde::Deserialize;

use crate::wisp::{ClothingVisual, PopulateWisp, SpawnWisp};

pub(crate) struct MisguidedPlugin;

impl Plugin for MisguidedPlugin {
    fn build(&self, app: &mut App) {
        app.add_actor::<SpawnMisguided>(ActorName::core("misguided"))
            .add_systems(Startup, init)
            .add_systems(
                FixedUpdate,
                spawn_blacksmith
                    .before(crate::wisp::populate_wisp)
                    .in_set(LevelSystemSet::Tick),
            );
    }
}

#[derive(Resource)]
struct MisguidedResources {
    hat: Handle<Scene>,
    name: Name,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SpawnMisguided {
    wisp: SpawnWisp,
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(MisguidedResources {
        hat: asset_server.load::<Scene>("actors/clothing/fun_hat.glb#Scene0"),
        name: Name::new("misguided"),
    });
}

fn spawn_blacksmith(
    mut commands: Commands,
    resources: Res<MisguidedResources>,
    mut writer: EventWriter<PopulateWisp>,
    mut spawn_requests: EventReader<SpawnActorEvent<SpawnMisguided>>,
) {
    for spawn in spawn_requests.read() {
        let entity = commands.spawn_empty().id();
        commands
            .entity(entity)
            .insert((Inventory::new(entity, 5), Interactable))
            .observe(on_interacted);
        writer.send(PopulateWisp(
            entity,
            resources.name.clone(),
            SpawnActorEvent::<SpawnWisp> {
                transform: spawn.transform,
                event: SpawnWisp {
                    hat: Some(ClothingVisual {
                        scene: resources.hat.clone(),
                        offset: Transform::default(),
                    }),
                    ..spawn.event.wisp.clone()
                },
            },
        ));
    }
}

fn on_interacted(
    trigger: Trigger<InteractedEvent>,
    query: Query<&ActiveDialogue>,
    mut commands: Commands,
    dialogues: Res<Dialogues>,
) {
    if query.contains(trigger.entity()) {
        //already a dialogue happening, don't cancel it.
        return;
    }
    let Some(introduction) = dialogues.dialogues.get("misguided.introduction") else {
        error!("dialogue not found!");
        return;
    };
    if let Some(mut ec) = commands.get_entity(trigger.entity()) {
        ec.insert(ActiveDialogue::new(introduction.clone(), trigger.user));
        info!("inserted dialogue!");
    }
}
