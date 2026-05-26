use bevy::prelude::*;
use bevy::render::{
    mesh::{Indices, PrimitiveTopology},
    render_asset::RenderAssetUsages,
};
use symbia_shared::{BlockType, ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

/// Génère un mesh voxel avec winding order CCW 100% compatible Bevy/WGPU.
/// ✅ Chaque face vérifiée par règle de la main droite (Cross-Product)
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

    let color = |b: BlockType| -> [f32;4] {
        match b {
            BlockType::Grass   => [0.20, 0.75, 0.30, 1.0],
            BlockType::Dirt    => [0.55, 0.35, 0.15, 1.0],
            BlockType::Stone   => [0.45, 0.45, 0.48, 1.0],
            BlockType::Bedrock => [0.15, 0.15, 0.18, 1.0],
            _                  => [1.0, 1.0, 1.0, 1.0],
        }
    };

    // ✅ Ajoute une face. v0,v1,v2,v3 DOIVENT être en ordre CCW vu de l'extérieur.
    // Indices [0,1,2] + [0,2,3] préservent le winding.
    let mut add_face = |v0, v1, v2, v3: [f32;3], n: [f32;3], col: [f32;4]| {
        let base = positions.len() as u32;
        positions.extend([v0, v1, v2, v3]);
        normals.extend([n, n, n, n]);
        uvs.extend([[0.0,0.0], [1.0,0.0], [1.0,1.0], [0.0,1.0]]);
        colors.extend([col, col, col, col]);
        indices.extend([base, base+1, base+2, base, base+2, base+3]);
    };

    for x in 0..CHUNK_SIZE_X {
        for z in 0..CHUNK_SIZE_Z {
            for y in 0..CHUNK_SIZE_Y {
                let b = chunk.get_block(x as usize, y as usize, z as usize).unwrap_or(BlockType::Air);
                if b == BlockType::Air { continue; }
                let c = color(b);
                let (fx, fy, fz) = (x as f32, y as f32, z as f32);

                // ── TOP (+Y) ── Normal (0,1,0)
                if get(x, y+1, z) == BlockType::Air {
                    add_face([fx, fy+1.0, fz+1.0], [fx+1.0, fy+1.0, fz+1.0], [fx+1.0, fy+1.0, fz], [fx, fy+1.0, fz], [0.0, 1.0, 0.0], c);
                }
                // ── BOTTOM (-Y) ── Normal (0,-1,0)
                if get(x, y-1, z) == BlockType::Air {
                    add_face([fx, fy, fz], [fx+1.0, fy, fz], [fx+1.0, fy, fz+1.0], [fx, fy, fz+1.0], [0.0, -1.0, 0.0], c);
                }
                // ── FRONT (-Z) ── Normal (0,0,-1)
                if get(x, y, z-1) == BlockType::Air {
                    add_face([fx+1.0, fy, fz], [fx, fy, fz], [fx, fy+1.0, fz], [fx+1.0, fy+1.0, fz], [0.0, 0.0, -1.0], c);
                }
                // ── BACK (+Z) ── Normal (0,0,1)
                if get(x, y, z+1) == BlockType::Air {
                    add_face([fx, fy, fz+1.0], [fx+1.0, fy, fz+1.0], [fx+1.0, fy+1.0, fz+1.0], [fx, fy+1.0, fz+1.0], [0.0, 0.0, 1.0], c);
                }
                // ── LEFT (-X) ── Normal (-1,0,0)
                if get(x-1, y, z) == BlockType::Air {
                    add_face([fx, fy, fz], [fx, fy, fz+1.0], [fx, fy+1.0, fz+1.0], [fx, fy+1.0, fz], [-1.0, 0.0, 0.0], c);
                }
                // ── RIGHT (+X) ── Normal (1,0,0)
                if get(x+1, y, z) == BlockType::Air {
                    add_face([fx+1.0, fy, fz+1.0], [fx+1.0, fy, fz], [fx+1.0, fy+1.0, fz], [fx+1.0, fy+1.0, fz+1.0], [1.0, 0.0, 0.0], c);
                }
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