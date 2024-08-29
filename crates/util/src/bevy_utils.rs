use bevy::{
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

#[derive(Component)]
pub struct TimedDespawner(pub Timer);

pub fn update_timed_despawner(
    mut commands: Commands,
    mut query: Query<(Entity, &mut TimedDespawner)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in query.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

//backport from bevy 0.13
pub fn cuboid(half_size: Vec3) -> Mesh {
    let min = -half_size;
    let max = half_size;

    // Suppose Y-up right hand, and camera look from +Z to -Z
    let vertices = &[
        // Front
        ([min.x, min.y, max.z], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([max.x, min.y, max.z], [0.0, 0.0, 1.0], [1.0, 0.0]),
        ([max.x, max.y, max.z], [0.0, 0.0, 1.0], [1.0, 1.0]),
        ([min.x, max.y, max.z], [0.0, 0.0, 1.0], [0.0, 1.0]),
        // Back
        ([min.x, max.y, min.z], [0.0, 0.0, -1.0], [1.0, 0.0]),
        ([max.x, max.y, min.z], [0.0, 0.0, -1.0], [0.0, 0.0]),
        ([max.x, min.y, min.z], [0.0, 0.0, -1.0], [0.0, 1.0]),
        ([min.x, min.y, min.z], [0.0, 0.0, -1.0], [1.0, 1.0]),
        // Right
        ([max.x, min.y, min.z], [1.0, 0.0, 0.0], [0.0, 0.0]),
        ([max.x, max.y, min.z], [1.0, 0.0, 0.0], [1.0, 0.0]),
        ([max.x, max.y, max.z], [1.0, 0.0, 0.0], [1.0, 1.0]),
        ([max.x, min.y, max.z], [1.0, 0.0, 0.0], [0.0, 1.0]),
        // Left
        ([min.x, min.y, max.z], [-1.0, 0.0, 0.0], [1.0, 0.0]),
        ([min.x, max.y, max.z], [-1.0, 0.0, 0.0], [0.0, 0.0]),
        ([min.x, max.y, min.z], [-1.0, 0.0, 0.0], [0.0, 1.0]),
        ([min.x, min.y, min.z], [-1.0, 0.0, 0.0], [1.0, 1.0]),
        // Top
        ([max.x, max.y, min.z], [0.0, 1.0, 0.0], [1.0, 0.0]),
        ([min.x, max.y, min.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ([min.x, max.y, max.z], [0.0, 1.0, 0.0], [0.0, 1.0]),
        ([max.x, max.y, max.z], [0.0, 1.0, 0.0], [1.0, 1.0]),
        // Bottom
        ([max.x, min.y, max.z], [0.0, -1.0, 0.0], [0.0, 0.0]),
        ([min.x, min.y, max.z], [0.0, -1.0, 0.0], [1.0, 0.0]),
        ([min.x, min.y, min.z], [0.0, -1.0, 0.0], [1.0, 1.0]),
        ([max.x, min.y, min.z], [0.0, -1.0, 0.0], [0.0, 1.0]),
    ];

    let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
    let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

    let indices = Indices::U32(vec![
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ]);

    Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(indices))
}
