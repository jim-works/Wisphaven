use bevy::{pbr::NotShadowCaster, prelude::*, utils::HashMap};

use crate::world::{events::BlockDamageSetEvent, BlockCoord, BlockDamage};

pub struct DamagedBlockPlugin;

impl Plugin for DamagedBlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_damaged_block_effect)
            .add_systems(Startup, init);
    }
}

const DAMAGE_PHASES: usize = 5;

#[derive(Component)]
struct DamagedBlockEffect;

#[derive(Resource)]
struct DamagedBlockResources {
    damages: HashMap<BlockCoord, Entity>,
    phases: Vec<Handle<StandardMaterial>>,
    mesh: Handle<Mesh>,
}

fn init(
    assets: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //cube needs to be slightly bigger than the block to avoid z-fighting
    let mesh = meshes.add(Cuboid::from_length(1.001));
    let mut phases = Vec::new();
    for i in 0..DAMAGE_PHASES {
        let image = assets.load(format!("textures/effects/break{}.png", i));
        phases.push(materials.add(StandardMaterial {
            base_color_texture: Some(image),
            alpha_mode: AlphaMode::Premultiplied,
            unlit: true,
            fog_enabled: false,
            ..default()
        }));
    }
    commands.insert_resource(DamagedBlockResources {
        damages: HashMap::new(),
        phases,
        mesh,
    })
}

fn damage_to_phase(damage: BlockDamage) -> usize {
    (damage.damage * DAMAGE_PHASES as f32) as usize
}

fn update_damaged_block_effect(
    mut reader: EventReader<BlockDamageSetEvent>,
    mut resources: ResMut<DamagedBlockResources>,
    mut commands: Commands,
    mut damage_query: Query<&mut Handle<StandardMaterial>, With<DamagedBlockEffect>>,
) {
    for BlockDamageSetEvent {
        block_position,
        damage,
        damager: _,
    } in reader.read()
    {
        if damage.damage <= 0.0 || damage.damage >= 1.0 {
            //block is either healed or broken, remove any damages that may be present
            if let Some(entity) = resources.damages.remove(block_position) {
                commands.entity(entity).despawn();
            }
            continue;
        }
        //we will either spawn or update a damage now
        let new_material = resources.phases[damage_to_phase(*damage)].clone();
        match resources.damages.get(block_position) {
            Some(entity) => {
                if let Ok(mut handle) = damage_query.get_mut(*entity) {
                    *handle = new_material;
                }
            }
            None => {
                let mesh = resources.mesh.clone();
                resources.damages.insert(
                    *block_position,
                    commands
                        .spawn((
                            PbrBundle {
                                mesh,
                                material: new_material,
                                transform: Transform::from_translation(block_position.center()),
                                ..default()
                            },
                            DamagedBlockEffect,
                            NotShadowCaster,
                        ))
                        .id(),
                );
            }
        }
    }
}
