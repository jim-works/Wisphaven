use bevy::prelude::*;

use crate::{items::ItemSystemSet, BlockCoord, Level, LevelSystemSet};

use super::{
    movement::{Acceleration, Mass},
    query::{raycast, Raycast},
    PhysicsSystemSet,
};

//springy grapple
pub struct GrapplePlugin;

impl Plugin for GrapplePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (update_grapple, update_force)
                .chain()
                .in_set(PhysicsSystemSet::Main),
        )
        .add_systems(
            Update,
            (shoot_grapple)
                .in_set(LevelSystemSet::Main)
                .after(ItemSystemSet::UsageProcessing),
        )
        .add_event::<UpdateAccelEvent>()
        .add_event::<ShootGrappleEvent>();
    }
}

#[derive(Clone, Copy)]
pub enum GrappleTarget {
    Block {
        block_coord: BlockCoord,
        anchor_pos: Vec3,
    },
    Entity {
        target: Entity,
        anchor_offset: Vec3, //local space to target entity
    },
}

#[derive(Component)]
pub struct Grappled {
    target: GrappleTarget,
    strength: f32, //ignores mass
    remove_distance: Option<f32>,
}

#[derive(Event)]
pub struct ShootGrappleEvent {
    pub ray: Raycast,
    pub owner: Entity,
    pub strength: f32,
    pub remove_distance: Option<f32>,
}

#[derive(Event, Clone, Copy)]
struct UpdateAccelEvent(Entity, Vec3);

fn shoot_grapple(
    mut event_reader: EventReader<ShootGrappleEvent>,
    level: Res<Level>,
    physics_query: Query<&crate::BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &super::collision::Aabb)>,
    mut commands: Commands,
) {
    for ShootGrappleEvent {
        ray,
        owner,
        strength,
        remove_distance,
    } in event_reader.read()
    {
        let Some(hit) = raycast(*ray, &level, &physics_query, &object_query, &[*owner]) else {
            continue;
        };
        let Some(mut ec) = commands.get_entity(*owner) else {
            continue;
        };
        match hit {
            super::query::RaycastHit::Block(coord, pos) => {
                ec.insert(Grappled {
                    target: GrappleTarget::Block {
                        block_coord: coord,
                        anchor_pos: pos.hit_pos,
                    },
                    strength: *strength,
                    remove_distance: *remove_distance,
                });
            }
            super::query::RaycastHit::Object(hit) => {
                let Ok((_, gtf, _)) = object_query.get(hit.entity) else {
                    continue;
                };
                ec.insert(Grappled {
                    target: GrappleTarget::Entity {
                        target: hit.entity,
                        anchor_offset: gtf.affine().inverse().transform_point3(hit.hit_pos),
                    },
                    strength: *strength,
                    remove_distance: *remove_distance,
                });
            }
        }
    }
}

//couldn't figure out paramset here, since there's two queries that need read access at the same time
fn update_grapple(
    grapple_query: Query<(Entity, &Grappled, &GlobalTransform, Option<&Mass>)>,
    hit_entity_query: Query<(&GlobalTransform, Option<&Mass>)>,
    level: Res<Level>,
    mut writer: EventWriter<UpdateAccelEvent>,
    mut commands: Commands,
    mut gizmo: Gizmos,
) {
    for (entity, grapple, gtf, mass_opt) in grapple_query.iter() {
        match grapple.target {
            GrappleTarget::Block {
                block_coord,
                anchor_pos,
            } => {
                if level.get_block_entity(block_coord).is_none()
                    || grapple
                        .remove_distance
                        .map(|limit| gtf.translation().distance(anchor_pos) <= limit)
                        .unwrap_or(false)
                {
                    info!("too close!");
                    //block removed or too close
                    if let Some(mut ec) = commands.get_entity(entity) {
                        ec.remove::<Grappled>();
                    }
                    continue;
                }
                let a = grapple_force(gtf.translation(), anchor_pos, grapple.strength, &mut gizmo);
                writer.send(UpdateAccelEvent(entity, a));
            }
            GrappleTarget::Entity {
                target,
                anchor_offset,
            } => {
                let Ok((target_gtf, target_mass_opt)) = hit_entity_query.get(target) else {
                    //entity removed
                    if let Some(mut ec) = commands.get_entity(entity) {
                        ec.remove::<Grappled>();
                    }
                    continue;
                };
                if grapple
                    .remove_distance
                    .map(|limit| gtf.translation().distance(target_gtf.translation()) <= limit)
                    .unwrap_or(false)
                {
                    info!("too close!");
                    //too close
                    if let Some(mut ec) = commands.get_entity(entity) {
                        ec.remove::<Grappled>();
                    }
                    continue;
                }
                let f = grapple_force(
                    gtf.translation(),
                    target_gtf.transform_point(anchor_offset),
                    grapple.strength,
                    &mut gizmo,
                );
                let mass_ratio = Mass(
                    mass_opt.copied().unwrap_or_default().0
                        / target_mass_opt.copied().unwrap_or_default().0,
                );
                let target_a = mass_ratio.get_force(-f * mass_ratio.0);
                writer.send(UpdateAccelEvent(entity, f));
                writer.send(UpdateAccelEvent(target, target_a));
            }
        }
    }
}

fn update_force(mut reader: EventReader<UpdateAccelEvent>, mut query: Query<&mut Acceleration>) {
    for UpdateAccelEvent(e, da) in reader.read() {
        if let Ok(mut a) = query.get_mut(*e) {
            a.0 += *da;
        }
    }
}

fn grapple_force(pos: Vec3, anchor_pos: Vec3, strength: f32, gizmos: &mut Gizmos) -> Vec3 {
    gizmos.line(pos, anchor_pos, Color::OLIVE);
    (anchor_pos - pos) * strength
}
