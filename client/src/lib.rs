use bevy::prelude::*;
use bevy::window::{Cursor, CursorGrabMode, PrimaryWindow};

mod chunk_mesh;
mod chunk_loader;
use chunk_loader::{ChunkWorld, update_chunk_world};
use symbia_shared::{BlockType, ChunkCoord, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};

#[derive(Component)]
pub struct PlayerCamera;

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

pub fn run_client() {
    let seed = std::env::var("SYMBIA_SEED").ok().and_then(|s| s.parse::<u32>().ok()).unwrap_or(42);
    println!("🌍 Symbia Client (seed={})", seed);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "🌿 Symbia – FPS + Voxel".into(),
                resolution: (1280., 720.).into(),
                resizable: true,
                cursor: Cursor { grab_mode: CursorGrabMode::Confined, visible: false, ..Default::default() },
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, setup_world)
        .add_systems(Update, (mouse_look, keyboard_with_collision, handle_block_interaction, toggle_cursor, update_chunk_world))
        .run();
}

fn setup_world(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    // ✅ Culling par défaut réactivé
    let mat = materials.add(StandardMaterial { base_color: Color::WHITE, perceptual_roughness: 0.9, ..Default::default() });
    commands.insert_resource(ChunkWorld::new(
        std::env::var("SYMBIA_SEED").ok().and_then(|s| s.parse::<u32>().ok()).unwrap_or(42), mat
    ));

    let controller = CameraController::default();
    let initial_rotation = Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
    
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 25.0, 0.0).with_rotation(initial_rotation),
            projection: Projection::Perspective(PerspectiveProjection { near: 0.01, ..Default::default() }),
            ..Default::default()
        },
        PlayerCamera, controller,
    ));
    
    commands.spawn(DirectionalLightBundle { 
        directional_light: DirectionalLight { illuminance: 3000.0, shadows_enabled: true, ..Default::default() }, 
        transform: Transform::from_xyz(20.0, 40.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y), 
        ..Default::default() 
    });
    println!("🎮 WASD+Mouse | Échap=Curseur | ClicG=Casser | ClicD=Poser");
}

fn mouse_look(_time: Res<Time>, mut query: Query<(&mut Transform, &mut CameraController), With<PlayerCamera>>, mut motion: EventReader<bevy::input::mouse::MouseMotion>, _mouse_input: Res<ButtonInput<bevy::input::mouse::MouseButton>>) {
    let (mut transform, mut ctrl) = match query.get_single_mut() { Ok(q) => q, Err(_) => return };
    let mut delta = Vec2::ZERO;
    for event in motion.read() { delta += event.delta; }
    if delta == Vec2::ZERO { return; }
    ctrl.yaw -= delta.x * ctrl.mouse_sensitivity;
    ctrl.pitch -= delta.y * ctrl.mouse_sensitivity;
    ctrl.pitch = ctrl.pitch.clamp(-ctrl.pitch_limit, ctrl.pitch_limit);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, ctrl.yaw, ctrl.pitch, 0.0);
}

// 🔍 Vérifie si un bloc monde est solide
fn is_block_solid(world: &ChunkWorld, x: i32, y: i32, z: i32) -> bool {
    if y < 0 || y >= CHUNK_SIZE_Y { return false; }
    let cx = x.div_euclid(CHUNK_SIZE_X);
    let cz = z.div_euclid(CHUNK_SIZE_Z);
    let coord = ChunkCoord { x: cx, z: cz };
    if let Some(chunk) = world.data.get(&coord) {
        let lx = (x.rem_euclid(CHUNK_SIZE_X)) as usize;
        let lz = (z.rem_euclid(CHUNK_SIZE_Z)) as usize;
        let b = chunk.blocks.get(chunk.index(lx, y as usize, lz)).copied().unwrap_or(BlockType::Air);
        b != BlockType::Air
    } else { false }
}

// ✅ Vérifie si une AABB (boîte) intersecte un bloc solide
fn check_aabb_collision(world: &ChunkWorld, min: Vec3, max: Vec3) -> bool {
    let min_x = min.x.floor() as i32;
    let max_x = max.x.floor() as i32;
    let min_y = min.y.floor() as i32;
    let max_y = max.y.floor() as i32;
    let min_z = min.z.floor() as i32;
    let max_z = max.z.floor() as i32;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                if is_block_solid(world, x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

// ⌨️ Déplacement avec collision AABB (résolution séparée X/Z/Y → glissement le long des murs)
fn keyboard_with_collision(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &CameraController), With<PlayerCamera>>,
    world: Res<ChunkWorld>,
    key_input: Res<ButtonInput<KeyCode>>,
) {
    let (mut transform, ctrl) = match query.get_single_mut() { Ok(q) => q, Err(_) => return };

    // Directions projetées sur le plan horizontal (évite de monter/descendre en marchant)
    let forward_3d = transform.rotation * Vec3::NEG_Z;
    let right_3d = transform.rotation * Vec3::X;
    let forward = Vec3::new(forward_3d.x, 0.0, forward_3d.z).normalize();
    let right = Vec3::new(right_3d.x, 0.0, right_3d.z).normalize();

    let mut move_x = 0.0;
    let mut move_z = 0.0;
    let mut move_y = 0.0;

    if key_input.pressed(KeyCode::KeyW) { move_x += forward.x; move_z += forward.z; }
    if key_input.pressed(KeyCode::KeyS) { move_x -= forward.x; move_z -= forward.z; }
    if key_input.pressed(KeyCode::KeyA) { move_x -= right.x; move_z -= right.z; }
    if key_input.pressed(KeyCode::KeyD) { move_x += right.x; move_z += right.z; }
    if key_input.pressed(KeyCode::Space) { move_y += 1.0; }
    if key_input.pressed(KeyCode::ControlLeft) || key_input.pressed(KeyCode::ControlRight) { move_y -= 1.0; }

    // Normaliser XZ pour vitesse constante en diagonale
    let len_xz = (move_x * move_x + move_z * move_z).sqrt();
    let speed_xy = if len_xz > 0.0 { ctrl.speed * time.delta_seconds() / len_xz } else { 0.0 };
    move_x *= speed_xy;
    move_z *= speed_xy;
    move_y *= ctrl.speed * time.delta_seconds();

    // Dimensions du joueur (boîte 0.6x1.7x0.6 centrée sur la caméra)
    let hw = 0.3;      // half-width
    let h_bot = 0.85;  // half-height bas
    let h_top = 0.85;  // half-height haut

    let mut pos = transform.translation;

    // ✅ Résolution axe par axe (permet de glisser le long des murs)
    // 1. Axe X
    pos.x += move_x;
    let min = Vec3::new(pos.x - hw, pos.y - h_bot, pos.z - hw);
    let max = Vec3::new(pos.x + hw, pos.y + h_top, pos.z + hw);
    if check_aabb_collision(&world, min, max) { pos.x -= move_x; }

    // 2. Axe Z
    pos.z += move_z;
    let min = Vec3::new(pos.x - hw, pos.y - h_bot, pos.z - hw);
    let max = Vec3::new(pos.x + hw, pos.y + h_top, pos.z + hw);
    if check_aabb_collision(&world, min, max) { pos.z -= move_z; }

    // 3. Axe Y
    pos.y += move_y;
    let min = Vec3::new(pos.x - hw, pos.y - h_bot, pos.z - hw);
    let max = Vec3::new(pos.x + hw, pos.y + h_top, pos.z + hw);
    if check_aabb_collision(&world, min, max) { pos.y -= move_y; }

    transform.translation = pos;
}

fn handle_block_interaction(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut world: ResMut<ChunkWorld>,
    query: Query<&Transform, With<PlayerCamera>>,
    mouse: Res<ButtonInput<bevy::input::mouse::MouseButton>>,
) {
    if mouse.just_pressed(bevy::input::mouse::MouseButton::Left) || mouse.just_pressed(bevy::input::mouse::MouseButton::Right) {
        let cam = match query.get_single() { Ok(q) => q, Err(_) => return };
        let origin = cam.translation;
        let dir = cam.rotation * Vec3::NEG_Z;
        let max_steps = 60;
        let mut hit = None;
        let mut prev: Option<(i32,i32,i32)> = None;

        for i in 0..max_steps {
            let t = i as f32 * 0.1;
            let p = origin + dir * t;
            let bx = p.x.floor() as i32;
            let by = p.y.floor() as i32;
            let bz = p.z.floor() as i32;
            if is_block_solid(&world, bx, by, bz) { hit = Some((bx, by, bz)); break; }
            prev = Some((bx, by, bz));
        }

        if mouse.just_pressed(bevy::input::mouse::MouseButton::Left) {
            if let Some((bx, by, bz)) = hit {
                world.modify_block(&mut commands, &mut meshes, bx, by, bz, BlockType::Air);
            }
        }
        if mouse.just_pressed(bevy::input::mouse::MouseButton::Right) {
            if let Some((px, py, pz)) = prev {
                if !is_block_solid(&world, px, py, pz) {
                    let place = if py <= 0 { BlockType::Stone } else { BlockType::Grass };
                    world.modify_block(&mut commands, &mut meshes, px, py, pz, place);
                }
            }
        }
    }
}

fn toggle_cursor(key_input: Res<ButtonInput<KeyCode>>, mut window: Query<&mut Window, With<PrimaryWindow>>, _q: Query<&CameraController, With<PlayerCamera>>, _lt: Local<bool>) {
    if key_input.just_pressed(KeyCode::Escape) {
        if let Ok(mut w) = window.get_single_mut() {
            if w.cursor.grab_mode == CursorGrabMode::Confined { w.cursor.grab_mode = CursorGrabMode::None; w.cursor.visible = true; }
            else { w.cursor.grab_mode = CursorGrabMode::Confined; w.cursor.visible = false; }
        }
    }
}