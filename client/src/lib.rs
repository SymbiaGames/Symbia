use bevy::prelude::*;
use bevy::animation::{AnimationClip, AnimationPlayer};
use bevy::window::{Cursor, CursorGrabMode, PrimaryWindow};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::texture::{ImageSampler, ImageSamplerDescriptor, ImageFilterMode};
use bevy::asset::AssetPlugin;
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;


// Modules locaux
mod chunk_mesh;
mod chunk_loader;
mod hud;
mod particles;

use chunk_loader::{ChunkWorld, update_chunk_world};
use hud::{spawn_hud, update_hud, draw_crosshair, HudConfig, SeedResource};
use particles::{ParticleEmitter, Lifetime, update_particles, emit_particles, despawn_with_lifetime};
use symbia_shared::{BlockType, ChunkCoord, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z, SymbiaNetMessage};
use symbia_net::P2PNode;

// ─────────────────────────────────────────────────────────────
// COMPOSANTS
// ─────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct PlayerBody; // Le corps physique qui se déplace et contient le modèle 3D

#[derive(Component)]
pub struct PlayerCamera; // La caméra, enfant du corps

#[derive(Component)]
pub struct PlayerPhysics {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub jump_cooldown: f32,
    pub is_jumping_intentional: bool,
}

impl Default for PlayerPhysics {
    fn default() -> Self { 
        Self { velocity: Vec3::ZERO, on_ground: false, jump_cooldown: 0.0, is_jumping_intentional: false } 
    }
}

#[derive(Component)]
pub struct CameraController {
    pub speed: f32,
    pub mouse_sensitivity: f32,
    pub pitch_limit: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraController {
    fn default() -> Self { 
        Self { speed: 5.0, mouse_sensitivity: 0.002, pitch_limit: 1.5, yaw: 0.0, pitch: -0.2 } 
    }
}

#[derive(Resource)]
pub struct P2PManager {
    pub node: Arc<Mutex<P2PNode>>,
    pub local_seed: u32,
    pub peer_seed: Option<u32>,
    pub last_block_timestamps: Arc<Mutex<HashMap<String, u64>>>,
}

#[derive(Event)]
pub struct P2PMessageEvent(pub SymbiaNetMessage);

static TOKIO_RUNTIME: std::sync::OnceLock<Runtime> = std::sync::OnceLock::new();

fn get_tokio_runtime() -> &'static Runtime {
    TOKIO_RUNTIME.get_or_init(|| Runtime::new().expect("❌ Échec création runtime Tokio"))
}

#[derive(Component)]
pub struct PlayerAnimations {
    pub idle: Handle<AnimationClip>,
    pub walk: Handle<AnimationClip>,
    pub back: Handle<AnimationClip>,   // 💡 AJOUTÉ
    pub jump: Handle<AnimationClip>,   // 💡 AJOUTÉ
    pub strafe_left: Handle<AnimationClip>,
    pub strafe_right: Handle<AnimationClip>,
}

//const GRAVITY: f32 = -20.0;
//const JUMP_VELOCITY: f32 = 9.0;
#[allow(dead_code)]
const GROUND_CHECK_DIST: f32 = 0.1;
//const TERMINAL_VELOCITY: f32 = -15.0;
//const JUMP_COOLDOWN: f32 = 0.2;

// 💡 NOUVELLES VALEURS POUR UN SAUT RÉALISTE (environ 1.4 bloc de haut)
const GRAVITY: f32 = -25.0;       // Augmentée pour une chute plus "lourde" et naturelle
const JUMP_VELOCITY: f32 = 7.0;   // Réduite pour ne pas sauter aussi haut qu'un immeuble
const TERMINAL_VELOCITY: f32 = -15.0;
const JUMP_COOLDOWN: f32 = 0.2;


// ─────────────────────────────────────────────────────────────
// POINT D'ENTRÉE
// ─────────────────────────────────────────────────────────────

pub fn run_client() {
    let seed = std::env::var("SYMBIA_SEED").ok().and_then(|s| s.parse::<u32>().ok()).unwrap_or(42);
    println!("🌍 Symbia Client (seed={})", seed);

    let _ = get_tokio_runtime();
    let p2p_node = get_tokio_runtime().block_on(async {
        P2PNode::new().expect("❌ Échec initialisation P2P")
    });

    let manager = P2PManager {
        node: Arc::new(Mutex::new(p2p_node)),
        local_seed: seed,
        peer_seed: None,
        last_block_timestamps: Arc::new(Mutex::new(HashMap::new())),
    };

    App::new()
        .add_plugins(DefaultPlugins
            .set(AssetPlugin { file_path: "../assets".into(), ..Default::default() })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "🌿 Symbia P2P".into(),
                    resolution: (1280., 720.).into(),
                    resizable: true,
                    cursor: Cursor { grab_mode: CursorGrabMode::None, visible: true, ..Default::default() },
                    focused: true,
                    ..Default::default()
                }),
                ..Default::default()
            })
        )
        .insert_resource(manager)
        .insert_resource(HudConfig::default())
        .insert_resource(SeedResource(seed))
        .add_event::<P2PMessageEvent>()
        .add_systems(Startup, (setup_world, spawn_hud))
        .add_systems(Update, (
            p2p_poll_network,
            p2p_handle_messages,
            manage_cursor,
            mouse_look,
            keyboard_with_physics,
            animate_player,
            handle_block_interaction,
            update_chunk_world,
            update_hud,
            draw_crosshair,
            update_particles,
            emit_particles,
            despawn_with_lifetime,
        ))
        .run();
}


// ─────────────────────────────────────────────────────────────
// SETUP DU MONDE (Version 100% épurée, zéro doublon)
// ─────────────────────────────────────────────────────────────
fn setup_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    // 1. Configuration de la texture (atlas de blocs)
    let atlas_handle = load_or_create_atlas_sync(&asset_server, &mut images);
    if let Some(img) = images.get_mut(&atlas_handle) {
        img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Nearest,
            min_filter: ImageFilterMode::Nearest,
            ..Default::default()
        });
    }

    let mat = materials.add(StandardMaterial {
        base_color: Color::rgb(1.2, 1.2, 1.2),
        base_color_texture: Some(atlas_handle),
        perceptual_roughness: 0.7,
        metallic: 0.0,
        reflectance: 0.1,
        alpha_mode: AlphaMode::Opaque,
        ..Default::default()
    });

    let seed = std::env::var("SYMBIA_SEED").ok().and_then(|s| s.parse::<u32>().ok()).unwrap_or(42);
    commands.insert_resource(ChunkWorld::new(seed, mat));

    let ctrl = CameraController::default();
    let initial_yaw = Quat::from_rotation_y(ctrl.yaw);

    // 2. CORPS DU JOUEUR (Parent)
    let player_body_entity = commands.spawn((
        PlayerBody,
        PlayerPhysics::default(),
        PlayerAnimations { 
            idle: asset_server.load("models/player_base.glb#Animation0"),
            walk: asset_server.load("models/player_base.glb#Animation1"),
            back: asset_server.load("models/player_base.glb#Animation2"),
            jump: asset_server.load("models/player_base.glb#Animation3"),
            strafe_left: asset_server.load("models/player_base.glb#Animation4"),
            strafe_right: asset_server.load("models/player_base.glb#Animation5"),
        },
        SpatialBundle {
            transform: Transform::from_xyz(0.0, 50.0, 0.0).with_rotation(initial_yaw),
            ..Default::default()
        },
    )).id();

    // 3. ENFANTS DU CORPS (UN SEUL MODÈLE + CAMÉRA, AUCUN AUTRE)
    commands.entity(player_body_entity).with_children(|parent| {
        
        // 🦸 MODÈLE UNIQUE (Tes réglages qui fonctionnent parfaitement)
        parent.spawn((
            SceneBundle {
                scene: asset_server.load("models/player_base.glb#Scene0"),
                transform: Transform::from_xyz(0.0, -0.9, 0.0)
                    .with_scale(Vec3::splat(0.5))
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                ..Default::default()
            },
        ));

        // 🎥 CAMÉRA 3e PERSONNE
        parent.spawn((
            PlayerCamera,
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 2.0, 5.0),
                projection: Projection::Perspective(PerspectiveProjection { 
                    near: 0.1, 
                    ..Default::default() 
                }),
                ..Default::default()
            },
            ctrl,
        ));
    });

    // 4. Lumières
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight { 
            illuminance: 8000.0, 
            shadows_enabled: true, 
            shadow_depth_bias: 0.02, 
            ..Default::default() 
        },
        transform: Transform::from_xyz(20.0, 40.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y), 
        ..Default::default()
    });

    println!("🎮 Clic gauche=Jouer | Échap=Pause | WASD+Space (Vue 3e personne)");
}

// ─────────────────────────────────────────────────────────────
// SYSTÈMES RÉSEAU
// ─────────────────────────────────────────────────────────────

fn p2p_poll_network(manager: Res<P2PManager>, mut p2p_events: EventWriter<P2PMessageEvent>) {
    let messages = get_tokio_runtime().block_on(async {
        if let Ok(mut node) = manager.node.lock() { node.poll_events() } else { Vec::new() }
    });
    for msg in messages { p2p_events.send(P2PMessageEvent(msg)); }
}

fn p2p_handle_messages(
    manager: Res<P2PManager>, mut world: ResMut<ChunkWorld>, mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>, mut p2p_events: EventReader<P2PMessageEvent>,
) {
    for event in p2p_events.read() {
        match &event.0 {
            SymbiaNetMessage::SeedSync { seed, .. } => {
                if manager.peer_seed.is_none() { println!("🌱 Seed distante: {}", seed); }
            }
            SymbiaNetMessage::BlockUpdate { x, y, z, block_id, timestamp, peer_id: _ } => {
                let key = format!("{},{},{}", x, y, z);
                if let Ok(mut ts_map) = manager.last_block_timestamps.lock() {
                    if let Some(&last) = ts_map.get(&key) { if *timestamp <= last { continue; } }
                    ts_map.insert(key, *timestamp);
                }
                if let Some(bt) = BlockType::from_u8(*block_id) {
                    world.modify_block(&mut commands, &mut meshes, *x, *y, *z, bt);
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// CONTRÔLES
// ─────────────────────────────────────────────────────────────

fn mouse_look(
    mut body_query: Query<&mut Transform, (With<PlayerBody>, Without<PlayerCamera>)>,
    mut camera_query: Query<(&mut Transform, &mut CameraController), (With<PlayerCamera>, Without<PlayerBody>)>,
    mut motion: EventReader<bevy::input::mouse::MouseMotion>,
) {
    let Ok(mut body_transform) = body_query.get_single_mut() else { return };
    let Ok((mut cam_transform, mut ctrl)) = camera_query.get_single_mut() else { return };

    let mut delta = Vec2::ZERO;
    for e in motion.read() { delta += e.delta; }
    if delta == Vec2::ZERO { return; }

    // 1. YAW (Gauche/Droite) : On fait tourner le CORPS du joueur
    ctrl.yaw -= delta.x * ctrl.mouse_sensitivity;
    body_transform.rotation = Quat::from_rotation_y(ctrl.yaw);

    // 2. PITCH (Haut/Bas) : On incline la CAMÉRA
    ctrl.pitch -= delta.y * ctrl.mouse_sensitivity;
    ctrl.pitch = ctrl.pitch.clamp(-ctrl.pitch_limit, ctrl.pitch_limit);
    cam_transform.rotation = Quat::from_rotation_x(ctrl.pitch);
}

fn manage_cursor(
    key_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<bevy::input::mouse::MouseButton>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mut is_captured: Local<bool>
) {
    let Ok(mut w) = window.get_single_mut() else { return };
    if !*is_captured && w.cursor.grab_mode == CursorGrabMode::Locked { *is_captured = true; }
    if key_input.just_pressed(KeyCode::Escape) {
        *is_captured = !*is_captured;
        if *is_captured { println!("▶️ Repris"); } else { println!("⏸️ Pause"); }
    }
    if !*is_captured && mouse_input.just_pressed(bevy::input::mouse::MouseButton::Left) { 
        *is_captured = true; println!("▶️ Repris"); 
    }
    if *is_captured { w.cursor.grab_mode = CursorGrabMode::Locked; w.cursor.visible = false; } 
    else { w.cursor.grab_mode = CursorGrabMode::None; w.cursor.visible = true; }
}

// ─────────────────────────────────────────────────────────────
// COLLISIONS
// ─────────────────────────────────────────────────────────────

fn is_block_solid(w: &ChunkWorld, x: i32, y: i32, z: i32) -> bool {
    if y < 0 || y >= CHUNK_SIZE_Y { return false; }
    let coord = ChunkCoord { x: x.div_euclid(CHUNK_SIZE_X), z: z.div_euclid(CHUNK_SIZE_Z) };
    if let Some(c) = w.data.get(&coord) {
        let lx = (x.rem_euclid(CHUNK_SIZE_X)) as usize; 
        let lz = (z.rem_euclid(CHUNK_SIZE_Z)) as usize;
        c.blocks.get(c.index(lx, y as usize, lz)).copied().unwrap_or(BlockType::Air) != BlockType::Air
    } else { false }
}


// ─────────────────────────────────────────────────────────────
// PHYSIQUE DU JOUEUR (Version robuste et découplée du visuel)
// ─────────────────────────────────────────────────────────────
fn keyboard_with_physics(
    time: Res<Time>,
    mut body_query: Query<(&mut Transform, &mut PlayerPhysics), (With<PlayerBody>, Without<PlayerCamera>)>,
    world: Res<ChunkWorld>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok((mut body_transform, mut p)) = body_query.get_single_mut() else { return };
    let dt = time.delta_seconds();

    // Direction du mouvement
    let fwd = (body_transform.rotation * Vec3::NEG_Z).normalize_or_zero();
    let flat_fwd = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
    let rt = (body_transform.rotation * Vec3::X).normalize_or_zero();
    let flat_rt = Vec3::new(rt.x, 0.0, rt.z).normalize_or_zero();

    let mut inp = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { inp += Vec2::new(flat_fwd.x, flat_fwd.z); }
    if keys.pressed(KeyCode::KeyS) { inp -= Vec2::new(flat_fwd.x, flat_fwd.z); }
    if keys.pressed(KeyCode::KeyA) { inp -= Vec2::new(flat_rt.x, flat_rt.z); }
    if keys.pressed(KeyCode::KeyD) { inp += Vec2::new(flat_rt.x, flat_rt.z); }

    let move_speed = 3.0; 

    if inp.length_squared() > 0.0 { 
        inp = inp.normalize(); 
        p.velocity.x = inp.x * move_speed; 
        p.velocity.z = inp.y * move_speed; 
    } else { 
        p.velocity.x *= 0.8; 
        p.velocity.z *= 0.8; 
        if p.velocity.x.abs() < 0.1 { p.velocity.x = 0.0; } 
        if p.velocity.z.abs() < 0.1 { p.velocity.z = 0.0; } 
    }

    p.jump_cooldown -= dt;
    if keys.just_pressed(KeyCode::Space) && p.on_ground && p.jump_cooldown <= 0.0 { 
        p.velocity.y = JUMP_VELOCITY; 
        p.on_ground = false; 
        p.jump_cooldown = JUMP_COOLDOWN; 
        p.is_jumping_intentional = true;
    }

    if !p.on_ground { 
        p.velocity.y += GRAVITY * dt;
        if p.velocity.y < TERMINAL_VELOCITY { p.velocity.y = TERMINAL_VELOCITY; } 
    }

    let mut pos = body_transform.translation; 
    let (hw, hb, ht) = (0.3, 0.9, 0.9); // Demi-largeur, demi-hauteur bas, demi-hauteur haut

    // 1. Collision X (Murs uniquement, le sol est ignoré)
    let next_x = pos.x + p.velocity.x * dt;
    if check_wall_collision(&world, next_x, pos.y, pos.z, hw, hb, ht) { 
        p.velocity.x = 0.0; 
    } else {
        pos.x = next_x;
    }

    // 2. Collision Z (Murs uniquement, le sol est ignoré)
    let next_z = pos.z + p.velocity.z * dt;
    if check_wall_collision(&world, pos.x, pos.y, next_z, hw, hb, ht) { 
        p.velocity.z = 0.0; 
    } else {
        pos.z = next_z;
    }

    // 3. Collision Y (Sol et Plafond séparés)
    let next_y = pos.y + p.velocity.y * dt;
    if p.velocity.y <= 0.0 {
        // On vérifie le bloc JUSTE EN DESSOUS des pieds (avec une petite marge de sécurité)
        let check_y = (next_y - hb - 0.05).floor() as i32;
        let cx = pos.x.floor() as i32;
        let cz = pos.z.floor() as i32;
        
        let mut hit_ground = false;
        // Vérification élargie pour ne pas passer à travers les arêtes de blocs
        if is_block_solid(&world, cx, check_y, cz) ||
           is_block_solid(&world, (pos.x + hw).floor() as i32, check_y, cz) ||
           is_block_solid(&world, (pos.x - hw).floor() as i32, check_y, cz) ||
           is_block_solid(&world, cx, check_y, (pos.z + hw).floor() as i32) ||
           is_block_solid(&world, cx, check_y, (pos.z - hw).floor() as i32) {
            hit_ground = true;
        }

        if hit_ground {
            // On replace le joueur exactement au-dessus du bloc
            pos.y = (check_y as f32) + 1.0 + hb;
            p.velocity.y = 0.0;
            p.on_ground = true;
            p.is_jumping_intentional = false;
        } else {
            pos.y = next_y;
            p.on_ground = false;
        }
    } else {
        // On monte : on vérifie le plafond au-dessus de la tête
        let check_y = (next_y + ht).floor() as i32;
        let cx = pos.x.floor() as i32;
        let cz = pos.z.floor() as i32;
        
        if is_block_solid(&world, cx, check_y, cz) {
            pos.y = (check_y as f32) - ht;
            p.velocity.y = 0.0;
        } else {
            pos.y = next_y;
        }
    }

    body_transform.translation = pos;
}

// 💡 NOUVELLE FONCTION : Vérifie les murs en IGNORANT le bloc du sol
fn check_wall_collision(w: &ChunkWorld, x: f32, y: f32, z: f32, hw: f32, hb: f32, ht: f32) -> bool {
    // On commence la vérification un peu au-dessus des pieds (y - hb + 0.99) 
    // pour que le sol sur lequel on marche ne soit pas considéré comme un mur bloquant.
    let min_y = (y - hb + 0.99).floor() as i32; 
    let max_y = (y + ht).floor() as i32;
    
    for cx in (x - hw).floor() as i32..=(x + hw).floor() as i32 {
        for cy in min_y..=max_y {
            for cz in (z - hw).floor() as i32..=(z + hw).floor() as i32 {
                if is_block_solid(w, cx, cy, cz) { return true; }
            }
        }
    }
    false
}


// ─────────────────────────────────────────────────────────────
// SYSTÈME D'ANIMATION DU JOUEUR (Version simple pour Bevy 0.13)
// ─────────────────────────────────────────────────────────────

fn animate_player(
    mut animation_players: Query<(Entity, &mut AnimationPlayer)>,
    player_data: Query<(&PlayerPhysics, &PlayerAnimations), With<PlayerBody>>,
    parents: Query<&Parent>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok((physics, anims)) = player_data.get_single() else { return };
    
    let is_jumping = !physics.on_ground;
    let is_moving = physics.velocity.length_squared() > 0.1;
    
    // Détection des mouvements purs (pour éviter de jouer "côté" si on appuie sur W+A en même temps)
    let pure_left = keys.pressed(KeyCode::KeyA) && !keys.pressed(KeyCode::KeyW) && !keys.pressed(KeyCode::KeyS);
    let pure_right = keys.pressed(KeyCode::KeyD) && !keys.pressed(KeyCode::KeyW) && !keys.pressed(KeyCode::KeyS);
    let pure_back = keys.pressed(KeyCode::KeyS) && !keys.pressed(KeyCode::KeyW) && !keys.pressed(KeyCode::KeyA) && !keys.pressed(KeyCode::KeyD);

    for (entity, mut anim_player) in animation_players.iter_mut() {
        let mut current = entity;
        let mut is_descendant = false;
        
        for _ in 0..10 {
            if let Ok(parent) = parents.get(current) {
                current = parent.get();
                if player_data.contains(current) {
                    is_descendant = true;
                    break;
                }
            } else {
                break;
            }
        }

        if is_descendant {
            if physics.is_jumping_intentional {
                anim_player.play(anims.jump.clone()).repeat().set_speed(0.8);
            
            } else if is_jumping {
                // 💡 SAUT : Vitesse normale pour un "petit saut"
                anim_player.play(anims.walk.clone()).repeat().set_speed(0.8);
            } else if is_moving {
                if pure_left {
                    // 💡 PAS DE CÔTÉ GAUCHE
                    anim_player.play(anims.strafe_left.clone()).repeat().set_speed(1.4);
                } else if pure_right {
                    // 💡 PAS DE CÔTÉ DROIT
                    anim_player.play(anims.strafe_right.clone()).repeat().set_speed(1.4);
                } else if pure_back {
                    // 💡 MARCHE ARRIÈRE
                    anim_player.play(anims.back.clone()).repeat().set_speed(1.4);
                } else {
                    // 💡 MARCHE AVANT (par défaut si on bouge)
                    anim_player.play(anims.walk.clone()).repeat().set_speed(1.4);
                }
            } else {
                // 💡 REPOS (IDLE)
                anim_player.play(anims.idle.clone()).repeat().set_speed(1.0);
            }
            
            break; 
        }
    }
}

// ─────────────────────────────────────────────────────────────
// INTERACTION BLOCS
// ─────────────────────────────────────────────────────────────

fn handle_block_interaction(
    mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut world: ResMut<ChunkWorld>,
    query: Query<&GlobalTransform, With<PlayerCamera>>,
    mouse: Res<ButtonInput<bevy::input::mouse::MouseButton>>, manager: Res<P2PManager>
) {
    if mouse.just_pressed(bevy::input::mouse::MouseButton::Left) || mouse.just_pressed(bevy::input::mouse::MouseButton::Right) {
        let Ok(cam) = query.get_single() else { return };
        let origin = cam.translation();
        let dir = cam.compute_transform().rotation * Vec3::NEG_Z;
        
        let mut hit = None;
        let mut prev: Option<(i32, i32, i32)> = None;
        for i in 0..60 {
            let p = origin + dir * (i as f32 * 0.1);
            let bx = p.x.floor() as i32; let by = p.y.floor() as i32; let bz = p.z.floor() as i32;
            if is_block_solid(&world, bx, by, bz) { hit = Some((bx, by, bz)); break; }
            prev = Some((bx, by, bz));
        }
        
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        let my_id = if let Ok(n) = manager.node.lock() { n.peer_id() } else { "unknown".into() };

        if mouse.just_pressed(bevy::input::mouse::MouseButton::Left) {
            if let Some((bx, by, bz)) = hit { 
                world.modify_block(&mut commands, &mut meshes, bx, by, bz, BlockType::Air);
                if let Ok(mut n) = manager.node.lock() {
                    n.broadcast(&SymbiaNetMessage::BlockUpdate { x: bx, y: by, z: bz, block_id: BlockType::Air as u8, timestamp: ts, peer_id: my_id.clone() });
                }
                commands.spawn((ParticleEmitter { particles_per_second: 20.0, spawn_timer: Timer::from_seconds(0.05, TimerMode::Repeating), velocity_range: (0.5, 2.0), color: Color::rgb(0.8, 0.7, 0.5) }, Transform::from_xyz(bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5), Lifetime(Timer::from_seconds(0.3, TimerMode::Once))));
            }
        }
        if mouse.just_pressed(bevy::input::mouse::MouseButton::Right) {
            if let Some((px, py, pz)) = prev {
                if !is_block_solid(&world, px, py, pz) {
                    let place = if py <= 0 { BlockType::Stone } else { BlockType::Grass };
                    world.modify_block(&mut commands, &mut meshes, px, py, pz, place);
                    if let Ok(mut n) = manager.node.lock() {
                        n.broadcast(&SymbiaNetMessage::BlockUpdate { x: px, y: py, z: pz, block_id: place as u8, timestamp: ts, peer_id: my_id.clone() });
                    }
                    commands.spawn((ParticleEmitter { particles_per_second: 15.0, spawn_timer: Timer::from_seconds(0.07, TimerMode::Repeating), velocity_range: (0.3, 1.5), color: Color::rgb(0.4, 0.8, 0.4) }, Transform::from_xyz(px as f32 + 0.5, py as f32 + 0.5, pz as f32 + 0.5), Lifetime(Timer::from_seconds(0.25, TimerMode::Once))));
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// ATLAS TEXTURES
// ─────────────────────────────────────────────────────────────

fn load_or_create_atlas_sync(asset_server: &AssetServer, images: &mut Assets<Image>) -> Handle<Image> {
    let path = "textures/atlas.png"; 
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let full = cwd.join("assets").join(path);
    if Path::new(&full).exists() { println!("🎨 Chargement: {}", full.display()); asset_server.load(path) }
    else { println!("⚠️ Fallback procédural"); create_block_atlas(images) }
}

fn create_block_atlas(images: &mut Assets<Image>) -> Handle<Image> {
    let size = 1024usize; let tile = 256usize; let mut data = vec![0u8; size*size*4];
    #[inline] fn set(d: &mut[u8], s:usize, x:usize, y:usize, r:u8, g:u8, b:u8) { if x < s && y < s { let i = (y * s + x) * 4; d[i] = r; d[i+1] = g; d[i+2] = b; d[i+3] = 255; } }
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tx, ty, 101,67,33); }}
    for ty in 0..(tile/4) { for tx in 0..tile { set(&mut data, size, tx, ty, 34,139,34); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile+tx, ty, 34,139,34); }}
    for ty in 0..tile { for tx in 0..tile { let _n=(((tx as usize).wrapping_mul(13)^(ty as usize).wrapping_mul(17)) &31) as u8; set(&mut data, size, tile*2+tx, ty, 50,150,50); }}
    for ty in 0..tile { for tx in 0..tile { let n=(((tx as usize).wrapping_mul(13)^(ty as usize).wrapping_mul(17)) &31) as u8; set(&mut data, size, tile*3+tx, ty, 101+n,67+n,33+n); }}
    for ty in 0..tile { for tx in 0..tile { let n=(((tx as usize).wrapping_mul(13)^(ty as usize).wrapping_mul(17)) &31) as u8; set(&mut data, size, tx, tile+ty, 120+n,120+n,125+n); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile+tx, tile+ty, 128,128,128); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*2+tx, tile+ty, 210,180,140); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*3+tx, tile+ty, 100,100,100); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tx, tile*2+ty, 40,40,45); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile+tx, tile*2+ty, 50,50,55); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*2+tx, tile*2+ty, 90,50,20); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*3+tx, tile*2+ty, 140,100,60); }}
    for ty in 0..tile { for tx in 0..tile { let n=(((tx as usize).wrapping_mul(13)^(ty as usize).wrapping_mul(17)) &31) as u8; set(&mut data, size, tx, tile*3+ty, 20+n,120+n,40+n); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile+tx, tile*3+ty, 50,180,50); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*2+tx, tile*3+ty, 255,255,255); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*3+tx, tile*3+ty, 240,240,240); }}
    let mut img = Image::new_fill(Extent3d { width: size as u32, height: size as u32, depth_or_array_layers: 1 }, TextureDimension::D2, &data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    img.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST; 
    images.add(img)
}