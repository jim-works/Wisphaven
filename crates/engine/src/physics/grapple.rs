use bevy::prelude::*;

use crate::{
    actors::ghost::UseHand,
    items::ItemSystemSet,
    world::{BlockCoord, Level, LevelSystemSet},
};

use super::{
    movement::{Acceleration, Mass, Velocity},
    query::{raycast, Raycast},
    PhysicsLevelSet,
};

//springy grapple
pub struct GrapplePlugin;

impl Plugin for GrapplePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(
                FixedUpdate,
                (update_grapple, update_force)
                    .chain()
                    .in_set(PhysicsLevelSet::Main),
            )
            .add_systems(
                Update,
                (shoot_grapple)
                    .in_set(LevelSystemSet::Main)
                    .after(ItemSystemSet::UsageProcessing),
            )
            .add_systems(Update, update_visual.in_set(LevelSystemSet::Main))
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
    max_speed: f32,
    remove_distance: Option<f32>,
    visual: Entity,
}

#[derive(Component)]
struct GrappleVisual {
    user: Entity,
    visual_origin: Entity,
    width: f32,
}

#[derive(Resource)]
struct VisualResources {
    material: Handle<StandardMaterial>,
    mesh: Handle<Mesh>,
}

#[derive(Event)]
pub struct ShootGrappleEvent {
    pub ray: Raycast,
    pub owner: Entity,
    pub strength: f32,
    pub max_speed: f32,
    pub remove_distance: Option<f32>,
}

#[derive(Event, Clone, Copy)]
struct UpdateAccelEvent(Entity, Vec3);

fn init(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.insert_resource(VisualResources {
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.33, 0.33, 0.33),
            ..Default::default()
        }),
        mesh: meshes.add(crate::util::bevy_utils::cuboid(Vec3::splat(0.5))),
    })
}

fn shoot_grapple(
    mut event_reader: EventReader<ShootGrappleEvent>,
    level: Res<Level>,
    hand_query: Query<&UseHand>,
    physics_query: Query<&crate::world::BlockPhysics>,
    object_query: Query<(Entity, &GlobalTransform, &super::collision::Aabb)>,
    mut commands: Commands,
) {
    for ShootGrappleEvent {
        ray,
        owner,
        strength,
        remove_distance,
        max_speed,
    } in event_reader.read()
    {
        let Some(hit) = raycast(*ray, &level, &physics_query, &object_query, &[*owner]) else {
            continue;
        };
        let visual = GrappleVisual {
            visual_origin: hand_query.get(*owner).map_or(*owner, |hand| hand.hand),
            user: *owner,
            width: 0.1,
        };
        let visual_entity = commands.spawn(visual).id();
        let Some(mut ec) = commands.get_entity(*owner) else {
            continue;
        };
        match hit {
            super::query::RaycastHit::Block(coord, pos) => {
                ec.try_insert(Grappled {
                    target: GrappleTarget::Block {
                        block_coord: coord,
                        anchor_pos: pos.hit_pos,
                    },
                    strength: *strength,
                    remove_distance: *remove_distance,
                    max_speed: *max_speed,
                    visual: visual_entity,
                });
            }
            super::query::RaycastHit::Object(hit) => {
                let Ok((_, gtf, _)) = object_query.get(hit.entity) else {
                    continue;
                };
                ec.try_insert(Grappled {
                    target: GrappleTarget::Entity {
                        target: hit.entity,
                        anchor_offset: gtf.affine().inverse().transform_point3(hit.hit_pos),
                    },
                    strength: *strength,
                    remove_distance: *remove_distance,
                    max_speed: *max_speed,
                    visual: visual_entity,
                });
            }
        }
    }
}

//couldn't figure out paramset here, since there's two queries that need read access at the same time
fn update_grapple(
    grapple_query: Query<(
        Entity,
        &Grappled,
        &GlobalTransform,
        &Velocity,
        Option<&Mass>,
    )>,
    hit_entity_query: Query<(&GlobalTransform, Option<&Mass>)>,
    level: Res<Level>,
    mut writer: EventWriter<UpdateAccelEvent>,
    mut commands: Commands,
) {
    for (entity, grapple, gtf, v, mass_opt) in grapple_query.iter() {
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
                    //block removed or too close
                    if let Some(mut ec) = commands.get_entity(entity) {
                        ec.remove::<Grappled>();
                    }
                    continue;
                }
                let a = grapple_force(
                    gtf.translation(),
                    anchor_pos,
                    grapple.strength,
                    v.0,
                    grapple.max_speed,
                );
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
                    if let Some(mut ec) = commands.get_entity(grapple.visual) {
                        ec.despawn();
                    }
                    continue;
                };
                if grapple
                    .remove_distance
                    .map(|limit| gtf.translation().distance(target_gtf.translation()) <= limit)
                    .unwrap_or(false)
                {
                    //too close
                    if let Some(mut ec) = commands.get_entity(entity) {
                        ec.remove::<Grappled>();
                    }
                    if let Some(mut ec) = commands.get_entity(grapple.visual) {
                        ec.despawn();
                    }
                    continue;
                }
                let f = grapple_force(
                    gtf.translation(),
                    target_gtf.transform_point(anchor_offset),
                    grapple.strength,
                    v.0,
                    grapple.max_speed,
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

fn grapple_force(
    pos: Vec3,
    anchor_pos: Vec3,
    strength: f32,
    current_v: Vec3,
    max_speed: f32,
) -> Vec3 {
    let line = (anchor_pos - pos).normalize_or_zero();
    let v_in_line = current_v.project_onto_normalized(line);
    if !v_in_line.is_finite() || v_in_line.length_squared() > max_speed * max_speed {
        Vec3::ZERO
    } else {
        line * strength
    }
}

fn update_visual(
    uninit_query: Query<(Entity, &GrappleVisual), Without<Transform>>,
    mut init_query: Query<(&mut Transform, &GrappleVisual)>,
    user_query: Query<(&GlobalTransform, &Grappled), Without<GrappleVisual>>,
    dest_query: Query<&GlobalTransform, Without<GrappleVisual>>,
    resources: Res<VisualResources>,
    mut commands: Commands,
) {
    let calc_tf = |user: Entity, visual_origin: Entity, width: f32| {
        if let Ok((owner_gtf, grapple)) = user_query.get(user) {
            let origin = if let Ok(gtf) = dest_query.get(visual_origin) {
                gtf.translation()
            } else {
                owner_gtf.translation()
            };
            let dest = match grapple.target {
                GrappleTarget::Block {
                    block_coord: _,
                    anchor_pos,
                } => anchor_pos,
                GrappleTarget::Entity {
                    target,
                    anchor_offset,
                } => {
                    if let Ok(gtf) = dest_query.get(target) {
                        gtf.transform_point(anchor_offset)
                    } else {
                        origin
                    }
                }
            };
            Transform::from_translation((origin + dest) / 2.)
                .with_scale(Vec3::new(width, width, origin.distance(dest)))
                .looking_at(origin, Vec3::Y)
        } else {
            return Transform::default();
        }
    };

    for (
        entity,
        GrappleVisual {
            visual_origin,
            user,
            width,
        },
    ) in uninit_query.iter()
    {
        commands.entity(entity).insert((
            Mesh3d(resources.mesh.clone()),
            MeshMaterial3d(resources.material.clone()),
            calc_tf(*user, *visual_origin, *width),
        ));
    }

    for (
        mut tf,
        GrappleVisual {
            visual_origin,
            user,
            width,
        },
    ) in init_query.iter_mut()
    {
        *tf = calc_tf(*user, *visual_origin, *width);
    }
}
