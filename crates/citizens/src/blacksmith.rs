use bevy::prelude::*;
use engine::{
    actors::{ActorName, BuildActorRegistry, SpawnActorEvent},
    items::{ItemName, ItemResources, ItemStack, inventory::Inventory},
};
use interfaces::scheduling::LevelSystemSet;
use serde::Deserialize;

use crate::wisp::{ClothingVisual, PopulateWisp, SpawnWisp};

pub(crate) struct BlacksmithPlugin;

impl Plugin for BlacksmithPlugin {
    fn build(&self, app: &mut App) {
        app.add_actor::<SpawnBlacksmith>(ActorName::core("blacksmith"))
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
struct BlacksmithResources {
    apron: Handle<Scene>,
    name: Name,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SpawnBlacksmith {
    wisp: SpawnWisp,
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(BlacksmithResources {
        apron: asset_server.load::<Scene>("actors/clothing/blacksmith_apron.glb#Scene0"),
        name: Name::new("blacksmith"),
    });
}

fn spawn_blacksmith(
    mut commands: Commands,
    resources: Res<BlacksmithResources>,
    mut writer: EventWriter<PopulateWisp>,
    mut spawn_requests: EventReader<SpawnActorEvent<SpawnBlacksmith>>,
    items: Res<ItemResources>,
) {
    for spawn in spawn_requests.read() {
        let entity = commands.spawn_empty().id();
        let mut inventory = Inventory::new(entity, 5);
        inventory.set_slot_no_events(
            0,
            ItemStack::new(
                items
                    .registry
                    .get_basic(&ItemName::core("rock_hammer"))
                    .unwrap(),
                1,
            ),
        );
        commands.entity(entity).insert(inventory);
        writer.send(PopulateWisp(
            entity,
            resources.name.clone(),
            SpawnActorEvent::<SpawnWisp> {
                transform: spawn.transform,
                event: SpawnWisp {
                    front: Some(ClothingVisual {
                        scene: resources.apron.clone(),
                        offset: Transform::default(),
                    }),
                    ..spawn.event.wisp.clone()
                },
            },
        ));
    }
}
