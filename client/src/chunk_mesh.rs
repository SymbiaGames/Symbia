use bevy::prelude::*;
use bevy::render::{
    mesh::{Indices, PrimitiveTopology},
    render_asset::RenderAssetUsages,
};
use symbia_shared::{BlockType, ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

pub fn build_chunk_mesh(chunk: &ChunkData) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    let get = |x: i32, y: i32, z: i32| -> BlockType {
        if x >= 0 && x < CHUNK_SIZE_X && y >= 0 && y < CHUNK_SIZE_Y && z >= 0 && z < CHUNK_SIZE_Z {
            chunk.get_block(x as usize, y as usize, z as usize).unwrap_or(BlockType::Air)
        } else { BlockType::Air }
    };

    // ✅ MAPPING FINAL - Ton atlas (origine en HAUT, index = col + row*4)
    let tile_index = |b: BlockType, is_top: bool, is_bottom: bool| -> usize {
        match b {
            BlockType::Grass => {
                if is_top { 1 }        // GrassTop (col1, row0)
                else if is_bottom { 3 } // Dirt (col3, row0)
                else { 0 }             // GrassSide (col0, row0)
            },
            BlockType::Dirt => 3,
            BlockType::Stone => 4,
            BlockType::Bedrock => 8,
            BlockType::Wood => 10,
            BlockType::Leaves => 12,
            _ => 0,
        }
    };

    let mut add_face = |v0, v1, v2, v3: [f32;3], n: [f32;3], tile: usize| {
        let base = positions.len() as u32;
        positions.extend([v0, v1, v2, v3]);
        normals.extend([n, n, n, n]);
        
        let tile_size = 0.25;
        let tx = (tile % 4) as f32;
        let ty = (tile / 4) as f32;
        let pad = 0.005;

        let u0 = tx * tile_size + pad;
        let u1 = (tx + 1.0) * tile_size - pad;
        
        // ✅ FLIP V SIMPLE : échanger v0 et v1 pour inverser la texture verticalement
        let v_raw_0 = ty * tile_size + pad;           // Bas de la tuile (dans l'image)
        let v_raw_1 = (ty + 1.0) * tile_size - pad;   // Haut de la tuile (dans l'image)
        
        // Bevy: v=0 est en BAS, mais ton image a y=0 en HAUT → on inverse
        let v0 = v_raw_1;  // Bas de la face = Haut de la tuile dans l'image
        let v1 = v_raw_0;  // Haut de la face = Bas de la tuile dans l'image

        // Quad CCW: [0,1,2] + [0,2,3] avec v0=bas, v1=haut
        uvs.extend([[u0, v0], [u1, v0], [u1, v1], [u0, v1]]);
        colors.extend([[1.0;4], [1.0;4], [1.0;4], [1.0;4]]);
        indices.extend([base, base+1, base+2, base, base+2, base+3]);
    };

    for x in 0..CHUNK_SIZE_X {
        for z in 0..CHUNK_SIZE_Z {
            for y in 0..CHUNK_SIZE_Y {
                let b = chunk.get_block(x as usize, y as usize, z as usize).unwrap_or(BlockType::Air);
                if b == BlockType::Air { continue; }
                let (fx, fy, fz) = (x as f32, y as f32, z as f32);

                if get(x, y+1, z) == BlockType::Air { add_face([fx, fy+1.0, fz+1.0], [fx+1.0, fy+1.0, fz+1.0], [fx+1.0, fy+1.0, fz], [fx, fy+1.0, fz], [0.0, 1.0, 0.0], tile_index(b, true, false)); }
                if get(x, y-1, z) == BlockType::Air { add_face([fx, fy, fz], [fx+1.0, fy, fz], [fx+1.0, fy, fz+1.0], [fx, fy, fz+1.0], [0.0, -1.0, 0.0], tile_index(b, false, true)); }
                if get(x, y, z-1) == BlockType::Air { add_face([fx+1.0, fy, fz], [fx, fy, fz], [fx, fy+1.0, fz], [fx+1.0, fy+1.0, fz], [0.0, 0.0, -1.0], tile_index(b, false, false)); }
                if get(x, y, z+1) == BlockType::Air { add_face([fx, fy, fz+1.0], [fx+1.0, fy, fz+1.0], [fx+1.0, fy+1.0, fz+1.0], [fx, fy+1.0, fz+1.0], [0.0, 0.0, 1.0], tile_index(b, false, false)); }
                if get(x-1, y, z) == BlockType::Air { add_face([fx, fy, fz], [fx, fy, fz+1.0], [fx, fy+1.0, fz+1.0], [fx, fy+1.0, fz], [-1.0, 0.0, 0.0], tile_index(b, false, false)); }
                if get(x+1, y, z) == BlockType::Air { add_face([fx+1.0, fy, fz+1.0], [fx+1.0, fy, fz], [fx+1.0, fy+1.0, fz], [fx+1.0, fy+1.0, fz+1.0], [1.0, 0.0, 0.0], tile_index(b, false, false)); }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}