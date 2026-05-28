use bevy::prelude::*;
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

// Imports locaux
use chunk_loader::{ChunkWorld, update_chunk_world};
use hud::{spawn_hud, update_hud, draw_crosshair, HudConfig, SeedResource};
use particles::{ParticleEmitter, Lifetime, update_particles, emit_particles, despawn_with_lifetime};

// Shared & Net
use symbia_shared::{BlockType, ChunkCoord, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z, SymbiaNetMessage};
use symbia_net::P2PNode;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct PlayerPhysics {
    pub velocity: Vec3,
    pub on_ground: bool,
    pub jump_cooldown: f32,
}
impl Default for PlayerPhysics {
    fn default() -> Self { Self { velocity: Vec3::ZERO, on_ground: false, jump_cooldown: 0.0 } }
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
    fn default() -> Self { Self { speed: 5.0, mouse_sensitivity: 0.002, pitch_limit: 1.5, yaw: 0.0, pitch: -0.2 } }
}

// 🌐 P2PManager Thread-Safe
// 🌐 P2PManager Thread-Safe (Arc<Mutex<>> requis pour Bevy)
#[derive(Resource)]
pub struct P2PManager {
    pub node: Arc<Mutex<P2PNode>>,
    pub local_seed: u32,
    pub peer_seed: Option<u32>,
    pub last_block_timestamps: Arc<Mutex<HashMap<String, u64>>>,
}

#[derive(Event)]
pub struct P2PMessageEvent(pub SymbiaNetMessage);

// ⏱️ Runtime Tokio global pour libp2p
static TOKIO_RUNTIME: std::sync::OnceLock<Runtime> = std::sync::OnceLock::new();

fn get_tokio_runtime() -> &'static Runtime {
    TOKIO_RUNTIME.get_or_init(|| {
        Runtime::new().expect("❌ Échec création runtime Tokio")
    })
}

// 🎮 Constantes de physique
const GRAVITY: f32 = -20.0;
const JUMP_VELOCITY: f32 = 9.0;
const GROUND_CHECK_DIST: f32 = 0.1;
const TERMINAL_VELOCITY: f32 = -15.0;
const JUMP_COOLDOWN: f32 = 0.2;



// ─────────────────────────────────────────────────────────────
// run_client() COMPLET
// ─────────────────────────────────────────────────────────────
pub fn run_client() {
    let seed = std::env::var("SYMBIA_SEED").ok().and_then(|s| s.parse::<u32>().ok()).unwrap_or(42);
    println!("🌍 Symbia Client (seed={})", seed);

    // ✅ Initialiser le runtime Tokio pour libp2p
    let _ = get_tokio_runtime();

    // ✅ Créer le nœud P2P dans le contexte du runtime
    let p2p_node = get_tokio_runtime().block_on(async {
        P2PNode::new().expect("❌ Échec initialisation P2P")
    });

    // ✅ Manager thread-safe
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
        
        // 🌐 Systèmes réseau (groupe 1)
        .add_systems(Update, (
            p2p_poll_network,
            p2p_handle_messages,
            manage_cursor,
        ))
        // 🎮 Systèmes de jeu (groupe 2)
        .add_systems(Update, (
            mouse_look,
            keyboard_with_physics,
            handle_block_interaction,
            update_chunk_world,
            // 🎨 HUD & Particules
            update_hud,
            draw_crosshair,
            update_particles,
            emit_particles,
            despawn_with_lifetime,
        ))
        .run();
}

fn setup_world(
    mut commands: Commands, 
    mut materials: ResMut<Assets<StandardMaterial>>, 
    mut images: ResMut<Assets<Image>>, 
    asset_server: Res<AssetServer>
) {
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
    let rot = Quat::from_euler(EulerRot::YXZ, ctrl.yaw, ctrl.pitch, 0.0);
    
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 35.0, 0.0).with_rotation(rot),
            projection: Projection::Perspective(PerspectiveProjection { near: 0.01, ..Default::default() }),
            ..Default::default()
        },
        PlayerCamera, 
        ctrl, 
        PlayerPhysics::default()
    ));

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
    
    commands.spawn(PointLightBundle {
        point_light: PointLight { 
            color: Color::rgb(0.8, 0.85, 1.0), 
            intensity: 200.0, 
            radius: 100.0, 
            ..Default::default() 
        },
        transform: Transform::from_xyz(0.0, 50.0, 0.0), 
        ..Default::default()
    });

    println!("🎮 Clic gauche=Jouer | Échap=Pause | WASD+Space");
}

// 🌐 Système de polling (appelé chaque frame par Bevy)
// 🌐 Poll le réseau P2P (avec runtime Tokio)
fn p2p_poll_network(
    manager: Res<P2PManager>,
    mut p2p_events: EventWriter<P2PMessageEvent>,
) {
    // Utilise le runtime global pour poller le swarm (non-bloquant pour Bevy)
    let messages = get_tokio_runtime().block_on(async {
        if let Ok(mut node) = manager.node.lock() {
            node.poll_events()
        } else {
            Vec::new()
        }
    });
    
    for msg in messages {
        p2p_events.send(P2PMessageEvent(msg));
    }
}

// 🌐 Traitement des messages
fn p2p_handle_messages(
    manager: Res<P2PManager>,
    mut world: ResMut<ChunkWorld>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut p2p_events: EventReader<P2PMessageEvent>,
) {
    for event in p2p_events.read() {
        match &event.0 {
            SymbiaNetMessage::SeedSync { seed, .. } => {
                if manager.peer_seed.is_none() {
                    println!("🌱 Seed distante: {}", seed);
                }
            }
            SymbiaNetMessage::BlockUpdate { x, y, z, block_id, timestamp, peer_id: _ } => {
                let key = format!("{},{},{}", x, y, z);
                
                // Vérifie le timestamp pour éviter les conflits
                if let Ok(mut ts_map) = manager.last_block_timestamps.lock() {
                    if let Some(&last) = ts_map.get(&key) {
                        if *timestamp <= last { 
                            continue; // Ignore les vieux messages
                        }
                    }
                    ts_map.insert(key, *timestamp);
                }
                
                // Applique la modification localement
                if let Some(bt) = BlockType::from_u8(*block_id) {
                    world.modify_block(&mut commands, &mut meshes, *x, *y, *z, bt);
                }
            }
        }
    }
}


fn mouse_look(_time: Res<Time>, mut query: Query<(&mut Transform, &mut CameraController), With<PlayerCamera>>, 
              mut motion: EventReader<bevy::input::mouse::MouseMotion>) {
    let (mut t, mut c) = match query.get_single_mut() { Ok(q) => q, Err(_) => return };
    let mut d = Vec2::ZERO; for e in motion.read() { d += e.delta; }
    if d == Vec2::ZERO { return; }
    c.yaw -= d.x * c.mouse_sensitivity; c.pitch -= d.y * c.mouse_sensitivity;
    c.pitch = c.pitch.clamp(-c.pitch_limit, c.pitch_limit);
    t.rotation = Quat::from_euler(EulerRot::YXZ, c.yaw, c.pitch, 0.0);
}

fn manage_cursor(
    key_input: Res<ButtonInput<KeyCode>>, 
    mouse_input: Res<ButtonInput<bevy::input::mouse::MouseButton>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>, 
    mut is_captured: Local<bool>
) {
    let Ok(mut w) = window.get_single_mut() else { return };
    
    if !*is_captured && w.cursor.grab_mode == CursorGrabMode::Locked { 
        *is_captured = true; 
    }
    
    if key_input.just_pressed(KeyCode::Escape) {
        *is_captured = !*is_captured;
        if *is_captured { 
            println!("▶️ Repris"); 
        } else { 
            println!("⏸️ Pause"); 
        }
    }
    
    if !*is_captured && mouse_input.just_pressed(bevy::input::mouse::MouseButton::Left) { 
        *is_captured = true; 
        println!("▶️ Repris"); 
    }
    
    if *is_captured { 
        w.cursor.grab_mode = CursorGrabMode::Locked; 
        w.cursor.visible = false; 
    } else { 
        w.cursor.grab_mode = CursorGrabMode::None; 
        w.cursor.visible = true; 
    }
}

fn is_block_solid(w: &ChunkWorld, x: i32, y: i32, z: i32) -> bool {
    if y < 0 || y >= CHUNK_SIZE_Y { return false; }
    let coord = ChunkCoord { x: x.div_euclid(CHUNK_SIZE_X), z: z.div_euclid(CHUNK_SIZE_Z) };
    if let Some(c) = w.data.get(&coord) {
        let lx = (x.rem_euclid(CHUNK_SIZE_X)) as usize; let lz = (z.rem_euclid(CHUNK_SIZE_Z)) as usize;
        c.blocks.get(c.index(lx, y as usize, lz)).copied().unwrap_or(BlockType::Air) != BlockType::Air
    } else { false }
}

fn check_aabb_collision(w: &ChunkWorld, min: Vec3, max: Vec3) -> bool {
    for x in min.x.floor() as i32..=max.x.floor() as i32 {
        for y in min.y.floor() as i32..=max.y.floor() as i32 {
            for z in min.z.floor() as i32..=max.z.floor() as i32 {
                if is_block_solid(w, x, y, z) { return true; }
            }
        }
    } false
}

fn keyboard_with_physics(
    time: Res<Time>, 
    mut q: Query<(&mut Transform, &mut CameraController, &mut PlayerPhysics), With<PlayerCamera>>, 
    world: Res<ChunkWorld>, 
    keys: Res<ButtonInput<KeyCode>>
) {
    let (mut t, ctrl, mut p) = match q.get_single_mut() { 
        Ok(x) => x, 
        Err(_) => return 
    };
    
    let dt = time.delta_seconds();
    let fwd = Vec3::new((t.rotation * Vec3::NEG_Z).x, 0., (t.rotation * Vec3::NEG_Z).z).normalize_or_zero();
    let rt = Vec3::new((t.rotation * Vec3::X).x, 0., (t.rotation * Vec3::X).z).normalize_or_zero();
    
    let mut inp = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { inp += Vec2::new(fwd.x, fwd.z); }
    if keys.pressed(KeyCode::KeyS) { inp -= Vec2::new(fwd.x, fwd.z); }
    if keys.pressed(KeyCode::KeyA) { inp -= Vec2::new(rt.x, rt.z); }
    if keys.pressed(KeyCode::KeyD) { inp += Vec2::new(rt.x, rt.z); }
    
    if inp.length_squared() > 0.0 { 
        inp = inp.normalize(); 
        p.velocity.x = inp.x * ctrl.speed; 
        p.velocity.z = inp.y * ctrl.speed; 
    } else { 
        p.velocity.x *= 0.9; 
        p.velocity.z *= 0.9; 
        if p.velocity.x.abs() < 0.1 { p.velocity.x = 0.0; } 
        if p.velocity.z.abs() < 0.1 { p.velocity.z = 0.0; } 
    }
    
    p.jump_cooldown -= dt;
    if keys.just_pressed(KeyCode::Space) && p.on_ground && p.jump_cooldown <= 0.0 { 
        p.velocity.y = JUMP_VELOCITY; 
        p.on_ground = false; 
        p.jump_cooldown = JUMP_COOLDOWN; 
    }
    
    if !p.on_ground { 
        p.velocity.y += GRAVITY * dt; 
        if p.velocity.y < TERMINAL_VELOCITY { 
            p.velocity.y = TERMINAL_VELOCITY; 
        } 
    }
    
    let mut pos = t.translation; 
    let (hw, hb, ht) = (0.3, 0.85, 0.85);
    
    // Collision axe par axe
    pos.x += p.velocity.x * dt; 
    if check_aabb_collision(&world, Vec3::new(pos.x-hw, pos.y-hb, pos.z-hw), Vec3::new(pos.x+hw, pos.y+ht, pos.z+hw)) { 
        pos.x -= p.velocity.x * dt; 
        p.velocity.x = 0.0; 
    }
    
    pos.z += p.velocity.z * dt; 
    if check_aabb_collision(&world, Vec3::new(pos.x-hw, pos.y-hb, pos.z-hw), Vec3::new(pos.x+hw, pos.y+ht, pos.z+hw)) { 
        pos.z -= p.velocity.z * dt; 
        p.velocity.z = 0.0; 
    }
    
    pos.y += p.velocity.y * dt;
    if check_aabb_collision(&world, Vec3::new(pos.x-hw, pos.y-hb, pos.z-hw), Vec3::new(pos.x+hw, pos.y+ht, pos.z+hw)) {
        if p.velocity.y < 0.0 { 
            pos.y = (pos.y - hb).floor() + hb + 1.0; 
            p.velocity.y = 0.0; 
            p.on_ground = true; 
        } else { 
            pos.y = (pos.y + ht).floor() + ht - 1.0; 
            p.velocity.y = 0.0; 
        }
    } else { 
        p.on_ground = false; 
    }
    
    // Ground check supplémentaire
    if !p.on_ground && p.velocity.y <= 0.0 { 
        let fp = Vec3::new(pos.x, pos.y - hb - GROUND_CHECK_DIST, pos.z); 
        if is_block_solid(&world, fp.x.floor() as i32, fp.y.floor() as i32, fp.z.floor() as i32) { 
            p.on_ground = true; 
        } 
    }
    
    t.translation = pos;
}

fn handle_block_interaction(
    mut commands: Commands, 
    mut meshes: ResMut<Assets<Mesh>>, 
    mut world: ResMut<ChunkWorld>,
    query: Query<&Transform, With<PlayerCamera>>, 
    mouse: Res<ButtonInput<bevy::input::mouse::MouseButton>>,
    manager: Res<P2PManager>
) {
    if mouse.just_pressed(bevy::input::mouse::MouseButton::Left) 
        || mouse.just_pressed(bevy::input::mouse::MouseButton::Right) 
    {
        let cam = match query.get_single() { 
            Ok(q) => q, 
            Err(_) => return 
        };
        
        let origin = cam.translation;
        let dir = cam.rotation * Vec3::NEG_Z;
        let mut hit = None;
        let mut prev: Option<(i32, i32, i32)> = None;
        
        // Raycast simple
        for i in 0..60 {
            let p = origin + dir * (i as f32 * 0.1);
            let bx = p.x.floor() as i32;
            let by = p.y.floor() as i32;
            let bz = p.z.floor() as i32;
            
            if is_block_solid(&world, bx, by, bz) { 
                hit = Some((bx, by, bz)); 
                break; 
            }
            prev = Some((bx, by, bz));
        }
        
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
            
        let my_id = if let Ok(n) = manager.node.lock() { 
            n.peer_id() 
        } else { 
            "unknown".into() 
        };

        // 🪨 Casser un bloc (clic gauche)
        if mouse.just_pressed(bevy::input::mouse::MouseButton::Left) {
            if let Some((bx, by, bz)) = hit { 
                world.modify_block(&mut commands, &mut meshes, bx, by, bz, BlockType::Air);
                
                // 🌐 Broadcast P2P
                if let Ok(mut n) = manager.node.lock() {
                    n.broadcast(&SymbiaNetMessage::BlockUpdate { 
                        x: bx, y: by, z: bz, 
                        block_id: BlockType::Air as u8, 
                        timestamp: ts, 
                        peer_id: my_id.clone() 
                    });
                }
                
                // 🎇 Particules de cassure
                commands.spawn((
                    ParticleEmitter {
                        particles_per_second: 20.0,
                        spawn_timer: Timer::from_seconds(0.05, TimerMode::Repeating),
                        velocity_range: (0.5, 2.0),
                        color: Color::rgb(0.8, 0.7, 0.5), // Couleur "poussière"
                    },
                    Transform::from_xyz(bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5),
                    Lifetime(Timer::from_seconds(0.3, TimerMode::Once)),
                ));
            }
        }
        
        // 🌱 Poser un bloc (clic droit)
        if mouse.just_pressed(bevy::input::mouse::MouseButton::Right) {
            if let Some((px, py, pz)) = prev {
                if !is_block_solid(&world, px, py, pz) {
                    let place = if py <= 0 { BlockType::Stone } else { BlockType::Grass };
                    world.modify_block(&mut commands, &mut meshes, px, py, pz, place);
                    
                    // 🌐 Broadcast P2P
                    if let Ok(mut n) = manager.node.lock() {
                        n.broadcast(&SymbiaNetMessage::BlockUpdate { 
                            x: px, y: py, z: pz, 
                            block_id: place as u8, 
                            timestamp: ts, 
                            peer_id: my_id.clone() 
                        });
                    }
                    
                    // 🎇 Particules de placement
                    commands.spawn((
                        ParticleEmitter {
                            particles_per_second: 15.0,
                            spawn_timer: Timer::from_seconds(0.07, TimerMode::Repeating),
                            velocity_range: (0.3, 1.5),
                            color: Color::rgb(0.4, 0.8, 0.4), // Couleur "herbe"
                        },
                        Transform::from_xyz(px as f32 + 0.5, py as f32 + 0.5, pz as f32 + 0.5),
                        Lifetime(Timer::from_seconds(0.25, TimerMode::Once)),
                    ));
                }
            }
        }
    }
}

fn load_or_create_atlas_sync(asset_server: &AssetServer, images: &mut Assets<Image>) -> Handle<Image> {
    let path = "textures/atlas.png"; let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let full = cwd.join("assets").join(path);
    if Path::new(&full).exists() { println!("🎨 Chargement: {}", full.display()); asset_server.load(path) }
    else { println!("⚠️ Fallback procédural"); create_block_atlas(images) }
}

fn create_block_atlas(images: &mut Assets<Image>) -> Handle<Image> {
    let size = 1024usize; let tile = 256usize; let mut data = vec![0u8; size*size*4];
    #[inline] fn set(d: &mut[u8], s:usize, x:usize, y:usize, r:u8, g:u8, b:u8) { if x<s && y<s { let i=(y*s+x)*4; d[i]=r; d[i+1]=g; d[i+2]=b; d[i+3]=255; } }
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tx, ty, 101,67,33); }}
    for ty in 0..(tile/4) { for tx in 0..tile { set(&mut data, size, tx, ty, 34,139,34); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile+tx, ty, 34,139,34); }}
    for ty in 0..tile { for tx in 0..tile { let _n=(((tx as usize).wrapping_mul(13)^(ty as usize).wrapping_mul(17))&31) as u8; set(&mut data, size, tile*2+tx, ty, 50,150,50); }}
    for ty in 0..tile { for tx in 0..tile { let n=(((tx as usize).wrapping_mul(13)^(ty as usize).wrapping_mul(17))&31) as u8; set(&mut data, size, tile*3+tx, ty, 101+n,67+n,33+n); }}
    for ty in 0..tile { for tx in 0..tile { let n=(((tx as usize).wrapping_mul(13)^(ty as usize).wrapping_mul(17))&31) as u8; set(&mut data, size, tx, tile+ty, 120+n,120+n,125+n); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile+tx, tile+ty, 128,128,128); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*2+tx, tile+ty, 210,180,140); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*3+tx, tile+ty, 100,100,100); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tx, tile*2+ty, 40,40,45); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile+tx, tile*2+ty, 50,50,55); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*2+tx, tile*2+ty, 90,50,20); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*3+tx, tile*2+ty, 140,100,60); }}
    for ty in 0..tile { for tx in 0..tile { let n=(((tx as usize).wrapping_mul(13)^(ty as usize).wrapping_mul(17))&31) as u8; set(&mut data, size, tx, tile*3+ty, 20+n,120+n,40+n); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile+tx, tile*3+ty, 50,180,50); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*2+tx, tile*3+ty, 255,255,255); }}
    for ty in 0..tile { for tx in 0..tile { set(&mut data, size, tile*3+tx, tile*3+ty, 240,240,240); }}
    let mut img = Image::new_fill(Extent3d{width:size as u32, height:size as u32, depth_or_array_layers:1}, TextureDimension::D2, &data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    img.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST; images.add(img)
}