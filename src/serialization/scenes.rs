use std::{fs::File, io::Write};

use bevy::{prelude::*, tasks::IoTaskPool};

use crate::items::{debug_items::PersonalityTester, weapons::MeleeWeaponItem, Item};

pub fn test_save(
    world: &mut World,
) {
    let mut scene_world = World::new();
    let mut dagger = MeleeWeaponItem::from_world(world);
    let mut item = Item::from_world(world);
    let tester = PersonalityTester::from_world(world);
    dagger.damage = 1.0;
    item.name = "test item".into();
    scene_world.spawn((
        MeleeWeaponItem {
            damage: 1.0,
            knockback: 2.0,
        },
        item,
        tester,
    ));

    // The TypeRegistry resource contains information about all registered types (including
    // components). This is used to construct scenes.
    let type_registry = world.resource::<AppTypeRegistry>();
    let scene = DynamicScene::from_world(&scene_world, type_registry);

    // Scenes can be serialized like this:
    let serialized_scene = scene.serialize_ron(type_registry).unwrap();

    // Showing the scene in the console
    info!("{}", serialized_scene);

    // Writing the scene to a new file. Using a task to avoid calling the filesystem APIs in a system
    // as they are blocking
    // This can't work in WASM as there is no filesystem access
    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            // Write the scene RON data to file
            File::create(format!("assets/items/test item.ron"))
                .and_then(|mut file| file.write(serialized_scene.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}