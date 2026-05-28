use bevy::prelude::*;
use symbia_shared::{BlockType, ChunkCoord, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

// ✅ Import des types depuis le crate parent (lib.rs)
use crate::{CameraController, ChunkWorld, PlayerCamera};

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct Crosshair;

#[derive(Resource)]
pub struct HudConfig {
    pub show_fps: bool,
    pub show_coords: bool,
    pub show_seed: bool,
}

impl Default for HudConfig {
    fn default() -> Self {
        Self {
            show_fps: true,
            show_coords: true,
            show_seed: true,
        }
    }
}

/// Spawn l'interface HUD au démarrage
/// Spawn l'interface HUD au démarrage
pub fn spawn_hud(mut commands: Commands, _config: Res<HudConfig>) {
    // UI Camera (2D par-dessus le jeu 3D)
    // ✅ Ordre 1 pour render APRÈS la caméra 3D (ordre 0 par défaut)
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                order: 1,  // ← IMPORTANT : render après la caméra 3D
                ..Default::default()
            },
            ..Default::default()
        },
    ));

    // Texte principal (coin haut-gauche)
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 14.0,
                color: Color::WHITE,
                ..Default::default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..Default::default()
        }),
        HudText,
    ));

    // Crosshair (centre écran) - optionnel, peut être fait avec Gizmos aussi
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Px(20.0),
                height: Val::Px(20.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect::all(Val::Px(-10.0)),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..Default::default()
        },
        Crosshair,
    ));
}

/// Met à jour le texte HUD chaque frame
pub fn update_hud(
    time: Res<Time>,
    mut query: Query<&mut Text, With<HudText>>,
    _config: Res<HudConfig>,  // ✅ Préfixe _ pour éviter le warning si non utilisé
    seed: Res<SeedResource>,
    player: Query<(&Transform, &CameraController), With<PlayerCamera>>,
    world: Option<Res<ChunkWorld>>,
) {
    let Ok(mut text) = query.get_single_mut() else { return };
    
    let mut lines = Vec::new();

    if _config.show_seed {
        lines.push(format!("🌱 Seed: {}", seed.0));
    }

    if _config.show_fps {
        let fps = 1.0 / time.delta_seconds();
        lines.push(format!("📊 FPS: {:.0}", fps));
    }

    if _config.show_coords {
        if let Ok((transform, _ctrl)) = player.get_single() {
            let pos = transform.translation;
            lines.push(format!("📍 Pos: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z));
            
            // Bloc visé (raycast simple)
            if let Some(world) = world {
                let dir = transform.rotation * Vec3::NEG_Z;
                for i in 0..20 {
                    let p = pos + dir * (i as f32 * 0.5);
                    let bx = p.x.floor() as i32;
                    let by = p.y.floor() as i32;
                    let bz = p.z.floor() as i32;
                    
                    if by >= 0 && by < CHUNK_SIZE_Y {
                        let coord = ChunkCoord { 
                            x: bx.div_euclid(CHUNK_SIZE_X), 
                            z: bz.div_euclid(CHUNK_SIZE_Z) 
                        };
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

/// Dessine le crosshair (simple croix)
pub fn draw_crosshair(
    mut gizmos: Gizmos, 
    player: Query<&Transform, With<PlayerCamera>>
) {
    let Ok(_transform) = player.get_single() else { return };
    
    // Croix simple au centre de l'écran (en espace écran 2D)
    let center = Vec2::new(0.0, 0.0);
    let size = 8.0;
    
    gizmos.line_2d(center + Vec2::new(-size, 0.0), center + Vec2::new(size, 0.0), Color::WHITE);
    gizmos.line_2d(center + Vec2::new(0.0, -size), center + Vec2::new(0.0, size), Color::WHITE);
}

/// Ressource pour la seed (à ajouter dans lib.rs si pas déjà fait)
#[derive(Resource)]
pub struct SeedResource(pub u32);