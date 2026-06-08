use bevy::prelude::*;
use std::collections::HashMap;
use symbia_shared::{ChunkCoord, ChunkData, BlockType, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};
use symbia_world::WorldGenerator;
use crate::chunk_mesh::build_chunk_mesh;
use crate::PlayerCamera;

#[derive(Component)]
pub struct ChunkEntity;

#[derive(Resource)]
pub struct ChunkWorld {
    pub generator: WorldGenerator,
    pub data: HashMap<ChunkCoord, ChunkData>,
    pub entities: HashMap<ChunkCoord, Entity>,
    pub material: Handle<StandardMaterial>,
}

impl ChunkWorld {
    pub fn new(seed: u32, material: Handle<StandardMaterial>) -> Self {
        Self { generator: WorldGenerator::new(seed), data: HashMap::new(), entities: HashMap::new(), material }
    }

    fn rebuild_chunk_mesh(&self, commands: &mut Commands, meshes: &mut Assets<Mesh>, coord: ChunkCoord) {
        if let Some(chunk_data) = self.data.get(&coord) {
            let mesh = build_chunk_mesh(chunk_data);
            let mesh_handle = meshes.add(mesh);
            if let Some(&entity) = self.entities.get(&coord) {
                commands.entity(entity).insert(mesh_handle);
            }
        }
    }

    pub fn modify_block(&mut self, commands: &mut Commands, meshes: &mut Assets<Mesh>, wx: i32, wy: i32, wz: i32, new_type: BlockType) {
        if wy < 0 || wy >= CHUNK_SIZE_Y { return; }
        let cx = wx.div_euclid(CHUNK_SIZE_X);
        let cz = wz.div_euclid(CHUNK_SIZE_Z);
        let coord = ChunkCoord { x: cx, z: cz };
        let lx = (wx.rem_euclid(CHUNK_SIZE_X)) as usize;
        let lz = (wz.rem_euclid(CHUNK_SIZE_Z)) as usize;
        let ly = wy as usize;

        if !self.data.contains_key(&coord) { self.data.insert(coord, self.generator.generate_chunk(coord)); }
        {
            let chunk = self.data.get_mut(&coord).unwrap();
            let idx = chunk.index(lx, ly, lz);
            chunk.blocks[idx] = new_type;
        }
        self.rebuild_chunk_mesh(commands, meshes, coord);

        let mut neighbors_to_rebuild = Vec::new();
        if lx == 0 { neighbors_to_rebuild.push(ChunkCoord { x: cx - 1, z: cz }); }
        if lx == CHUNK_SIZE_X as usize - 1 { neighbors_to_rebuild.push(ChunkCoord { x: cx + 1, z: cz }); }
        if lz == 0 { neighbors_to_rebuild.push(ChunkCoord { x: cx, z: cz - 1 }); }
        if lz == CHUNK_SIZE_Z as usize - 1 { neighbors_to_rebuild.push(ChunkCoord { x: cx, z: cz + 1 }); }

        for neighbor_coord in neighbors_to_rebuild {
            if self.data.contains_key(&neighbor_coord) {
                self.rebuild_chunk_mesh(commands, meshes, neighbor_coord);
            }
        }
    }
}

pub fn update_chunk_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<ChunkWorld>,
    player: Query<&GlobalTransform, With<PlayerCamera>>,
    time: Res<Time>,
    mut last_check: Local<f32>,
) {
    if time.elapsed_seconds() - *last_check < 0.2 { return; }
    *last_check = time.elapsed_seconds();
    
    let Ok(cam) = player.get_single() else { return };
    
    let cx = (cam.translation().x / CHUNK_SIZE_X as f32).floor() as i32;
    let cz = (cam.translation().z / CHUNK_SIZE_Z as f32).floor() as i32;
    let center = ChunkCoord { x: cx, z: cz };
    let spawn_dist = 2;
    let unload_dist = 4;

    for dx in -spawn_dist..=spawn_dist {
        for dz in -spawn_dist..=spawn_dist {
            let coord = ChunkCoord { x: center.x + dx, z: center.z + dz };
            if world.entities.contains_key(&coord) { continue; }

            let chunk_data = world.generator.generate_chunk(coord);
            world.data.insert(coord, chunk_data.clone());
            let mesh = build_chunk_mesh(&chunk_data);
            let mesh_handle = meshes.add(mesh);
            let offset = Vec3::new(coord.x as f32 * CHUNK_SIZE_X as f32, 0.0, coord.z as f32 * CHUNK_SIZE_Z as f32);
            
            let entity = commands.spawn((
                PbrBundle { mesh: mesh_handle, material: world.material.clone(), transform: Transform::from_translation(offset), ..Default::default() },
                ChunkEntity,
            )).id();
            world.entities.insert(coord, entity);
        }
    }

    let to_remove: Vec<ChunkCoord> = world.entities.keys()
        .filter(|c| (c.x - center.x).abs() > unload_dist || (c.z - center.z).abs() > unload_dist)
        .cloned()
        .collect();
    for coord in to_remove {
        if let Some(entity) = world.entities.remove(&coord) {
            commands.entity(entity).despawn();
            world.data.remove(&coord);
        }
    }
}