use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────
// CONSTANTES DE CHUNK
// ─────────────────────────────────────────────────────────────
pub const CHUNK_SIZE_X: i32 = 16;
pub const CHUNK_SIZE_Y: i32 = 64;
pub const CHUNK_SIZE_Z: i32 = 16;

// ─────────────────────────────────────────────────────────────
// TYPES DE BLOCS
// ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    Air = 0,
    Stone = 1,
    Dirt = 2,
    Grass = 3,
    Wood = 4,
    Leaves = 5,
    Bedrock = 6,
}

impl BlockType {
    pub fn is_solid(&self) -> bool { !matches!(self, BlockType::Air) }
    pub fn from_u8(id: u8) -> Option<Self> {
        match id {
            0 => Some(BlockType::Air), 1 => Some(BlockType::Stone), 2 => Some(BlockType::Dirt),
            3 => Some(BlockType::Grass), 4 => Some(BlockType::Wood), 5 => Some(BlockType::Leaves),
            6 => Some(BlockType::Bedrock), _ => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// COORDONNÉES
// ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockCoord { pub x: i32, pub y: i32, pub z: i32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord { pub x: i32, pub z: i32 }

// ─────────────────────────────────────────────────────────────
// DONNÉES DE CHUNK (déplacé ici depuis world/)
// ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub coord: ChunkCoord,
    pub blocks: Vec<BlockType>,
}

impl ChunkData {
    pub fn empty(coord: ChunkCoord) -> Self {
        let total = (CHUNK_SIZE_X * CHUNK_SIZE_Y * CHUNK_SIZE_Z) as usize;
        Self { coord, blocks: vec![BlockType::Air; total] }
    }
    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        x + z * CHUNK_SIZE_X as usize + y * (CHUNK_SIZE_X as usize * CHUNK_SIZE_Z as usize)
    }
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> Option<BlockType> {
        if x < CHUNK_SIZE_X as usize && y < CHUNK_SIZE_Y as usize && z < CHUNK_SIZE_Z as usize {
            Some(self.blocks[self.index(x, y, z)])
        } else { None }
    }
}
