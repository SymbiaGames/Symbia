use bevy::prelude::*;
use bevy::math::primitives::Sphere as MathSphere; // ← Nouvelle API Bevy 0.13

#[derive(Component)]
pub struct Particle {
    pub velocity: Vec3,
    pub lifetime: Timer,
    #[allow(dead_code)] // ← Supprime le warning si on ne l'utilise pas encore
    pub color: Color,
}

#[derive(Component)]
pub struct ParticleEmitter {
    #[allow(dead_code)] // ← Supprime le warning (réservé pour futur usage)
    pub particles_per_second: f32,
    pub spawn_timer: Timer,
    pub velocity_range: (f32, f32),
    pub color: Color,
}

/// Spawn une particule individuelle
pub fn spawn_particle_bundle(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    velocity: Vec3,
    color: Color,
    lifetime: f32,
) {
    // ✅ Bevy 0.13+ : Utiliser Sphere de bevy::math::primitives
    let mesh = meshes.add(Mesh::from(MathSphere { radius: 0.0375 }));
    
    let mat = materials.add(StandardMaterial {
        base_color: color,
        emissive: color * 0.3,
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    });

    commands.spawn((
        PbrBundle {
            mesh,
            material: mat,
            transform: Transform::from_translation(position),
            ..Default::default()
        },
        Particle {
            velocity,
            lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
            color,
        },
    ));
}

/// Met à jour les particules existantes
pub fn update_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Transform, &mut Particle)>,
) {
    for (entity, mut transform, mut particle) in particles.iter_mut() {
        particle.lifetime.tick(time.delta());
        
        if particle.lifetime.finished() {
            commands.entity(entity).despawn();
            continue;
        }

        // Mise à jour de la position
        transform.translation += particle.velocity * time.delta_seconds();
        
        // Gravité légère pour les particules
        particle.velocity.y -= 2.0 * time.delta_seconds();
    }
}

/// Émet des particules depuis un émetteur
pub fn emit_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut emitters: Query<(&mut ParticleEmitter, &Transform)>,
) {
    for (mut emitter, transform) in emitters.iter_mut() {
        emitter.spawn_timer.tick(time.delta());
        
        if emitter.spawn_timer.finished() {
            let pos = transform.translation;
            
            // Direction aléatoire dans un cône
            let speed = rand::random::<f32>() * (emitter.velocity_range.1 - emitter.velocity_range.0) 
                      + emitter.velocity_range.0;
            let angle = rand::random::<f32>() * std::f32::consts::TAU;
            
            let velocity = Vec3::new(
                speed * angle.cos(),
                speed * 0.5 + 1.0, // Un peu vers le haut
                speed * angle.sin(),
            );
            
            spawn_particle_bundle(
                &mut commands,
                &mut meshes,
                &mut materials,
                pos,
                velocity,
                emitter.color,
                0.5, // 0.5 seconde de vie
            );
        }
    }
}

/// Despawne les entités avec un Lifetime
#[derive(Component)]
pub struct Lifetime(pub Timer);

pub fn despawn_with_lifetime(
    time: Res<Time>,
    mut commands: Commands,
    mut entities: Query<(Entity, &mut Lifetime)>,
) {
    for (entity, mut lifetime) in entities.iter_mut() {
        lifetime.0.tick(time.delta());
        if lifetime.0.finished() {
            commands.entity(entity).despawn();
        }
    }
}