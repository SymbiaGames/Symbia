use bevy::prelude::*;
use symbia_shared::{BlockType, ChunkCoord, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};
use crate::{ChunkWorld, PlayerCamera};

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
#[allow(dead_code)]
pub struct Crosshair;

#[derive(Resource)]
pub struct HudConfig {
    pub show_fps: bool,
    pub show_coords: bool,
    pub show_seed: bool,
}

impl Default for HudConfig {
    fn default() -> Self {
        Self { show_fps: true, show_coords: true, show_seed: true }
    }
}

pub fn spawn_hud(mut commands: Commands, _config: Res<HudConfig>) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera { order: 1, ..Default::default() },
            ..Default::default()
        },
    ));
    commands.spawn((
        TextBundle::from_section("", TextStyle { font_size: 14.0, color: Color::WHITE, ..Default::default() })
            .with_style(Style { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(10.0), ..Default::default() }),
        HudText,
    ));
}

pub fn update_hud(
    time: Res<Time>,
    mut query: Query<&mut Text, With<HudText>>,
    _config: Res<HudConfig>,
    seed: Res<SeedResource>,
    player: Query<&GlobalTransform, With<PlayerCamera>>,
    world: Option<Res<ChunkWorld>>,
) {
    let Ok(mut text) = query.get_single_mut() else { return };
    let mut lines = Vec::new();

    if _config.show_seed { lines.push(format!("🌱 Seed: {}", seed.0)); }
    if _config.show_fps { lines.push(format!("📊 FPS: {:.0}", 1.0 / time.delta_seconds())); }

    if _config.show_coords {
        if let Ok(global_transform) = player.get_single() {
            let pos = global_transform.translation();
            lines.push(format!("📍 Pos: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z));
            
            if let Some(world) = world {
                let dir = global_transform.compute_transform().rotation * Vec3::NEG_Z;
                for i in 0..20 {
                    let p = pos + dir * (i as f32 * 0.5);
                    let bx = p.x.floor() as i32;
                    let by = p.y.floor() as i32;
                    let bz = p.z.floor() as i32;
                    
                    if by >= 0 && by < CHUNK_SIZE_Y {
                        let coord = ChunkCoord { x: bx.div_euclid(CHUNK_SIZE_X), z: bz.div_euclid(CHUNK_SIZE_Z) };
                        if let Some(chunk) = world.data.get(&coord) {
                            let lx = (bx.rem_euclid(CHUNK_SIZE_X)) as usize;
                            let lz = (bz.rem_euclid(CHUNK_SIZE_Z)) as usize;
                            if let Some(block) = chunk.blocks.get(chunk.index(lx, by as usize, lz)) {
                                if *block != BlockType::Air {
                                    lines.push(format!("🎯 Bloc: ({},{},{}) {:?}", bx, by, bz, block));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    text.sections[0].value = lines.join("\n");
}

pub fn draw_crosshair(mut gizmos: Gizmos, player: Query<&GlobalTransform, With<PlayerCamera>>) {
    let Ok(_) = player.get_single() else { return };
    let center = Vec2::new(0.0, 0.0);
    let size = 8.0;
    gizmos.line_2d(center + Vec2::new(-size, 0.0), center + Vec2::new(size, 0.0), Color::WHITE);
    gizmos.line_2d(center + Vec2::new(0.0, -size), center + Vec2::new(0.0, size), Color::WHITE);
}

#[derive(Resource)]
pub struct SeedResource(pub u32);