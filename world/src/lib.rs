use noise::{NoiseFn, Perlin};
use symbia_shared::{BlockType, ChunkCoord, ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

pub struct WorldGenerator {
    pub seed: u32,
    height_noise: Perlin,
}

impl WorldGenerator {
    pub fn new(seed: u32) -> Self {
        Self { seed, height_noise: Perlin::new(seed) }
    }
    fn get_terrain_height(&self, wx: i32, wz: i32) -> i32 {
        let scale = 0.015;
        let normalized = self.height_noise.get([wx as f64 * scale, wz as f64 * scale]) * 0.5 + 0.5;
        (normalized * 30.0 + 10.0) as i32
    }
    pub fn generate_chunk(&self, coord: ChunkCoord) -> ChunkData {
        let mut chunk = ChunkData::empty(coord);
        for lx in 0..CHUNK_SIZE_X {
            for lz in 0..CHUNK_SIZE_Z {
                let wx = coord.x * CHUNK_SIZE_X + lx;
                let wz = coord.z * CHUNK_SIZE_Z + lz;
                let surface_y = self.get_terrain_height(wx, wz);
                for ly in 0..CHUNK_SIZE_Y {
                    let idx = chunk.index(lx as usize, ly as usize, lz as usize);
                    let wy = ly as i32;
                    chunk.blocks[idx] = if wy <= 2 {
                        BlockType::Bedrock
                    } else if wy < surface_y - 4 {
                        BlockType::Stone
                    } else if wy < surface_y - 1 {
                        BlockType::Dirt
                    } else if wy == surface_y - 1 {
                        BlockType::Grass
                    } else {
                        BlockType::Air
                    };
                }
            }
        }
        chunk
    }
}
