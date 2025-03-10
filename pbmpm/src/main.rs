use bevy::prelude::*;
use glam::Vec2;
use std::collections::HashMap;
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Gravity(Vec2::new(0.0, -9.8)))
        .insert_resource(BounceDampening(0.8))
        .insert_resource(WindowSize { width: 400.0, height: 300.0 })
        .insert_resource(Grid::new(20.0))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_window_size, update_particles, update_grid, resolve_collisions))
        .run();
}

#[derive(Resource)]
struct Gravity(Vec2);

#[derive(Resource)]
struct BounceDampening(f32);

#[derive(Resource)]
struct WindowSize {
    width: f32,
    height: f32,
}

#[derive(Component)]
struct Particle {
    velocity: Vec2,
}

#[derive(Resource)]
struct Grid {
    cell_size: f32,
    cells: HashMap<(i32, i32), GridCell>,
    previous_velocities: HashMap<(i32, i32), Vec2>,
}


#[derive(Default, Clone, Copy)]
struct GridCell {
    velocity: Vec2,
    mass: f32,
}

impl Grid {
    fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            previous_velocities: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.previous_velocities.clear(); // Reset previous velocities
        for (cell_idx, cell) in &self.cells {
            self.previous_velocities.insert(*cell_idx, cell.velocity); // Store last frame's velocity
        }
        self.cells.clear(); // Reset grid
    }

    fn world_to_cell(&self, position: Vec2) -> (i32, i32) {
        (
            (position.x / self.cell_size) as i32,
            (position.y / self.cell_size) as i32,
        )
    }
}


fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let num_particles = 10;
    let spawn_radius = 50.0;

    for i in 0..num_particles {
        let angle = (i as f32 / num_particles as f32) * 2.0 * PI;
        let position = Vec2::new(spawn_radius * angle.cos(), spawn_radius * angle.sin());
        let initial_velocity = Vec2::new((i as f32 - 5.0) * 5.0, 50.0);

        commands.spawn((
            Particle {
                velocity: initial_velocity,
            },
            Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::splat(5.0)),
                ..Default::default()
            },
            Transform::from_translation(Vec3::new(position.x, position.y, 0.0)),
            Visibility::Visible,
        ));
    }
}

fn update_window_size(
    mut window_size: ResMut<WindowSize>,
    mut resize_events: EventReader<bevy::window::WindowResized>,
) {
    for event in resize_events.read() {
        window_size.width = event.width / 2.0;
        window_size.height = event.height / 2.0;
    }
}

fn update_particles(
    mut query: Query<(&mut Particle, &mut Transform, &Sprite)>,
    window_size: Res<WindowSize>,
    gravity: Res<Gravity>,
    bounce_dampening: Res<BounceDampening>,
    time: Res<Time>,
) {
    let delta_time = time.delta_secs();

    for (mut particle, mut transform, sprite) in &mut query {
        particle.velocity += gravity.0 * delta_time;

        let particle_size = sprite.custom_size.unwrap_or(Vec2::new(5.0, 5.0));
        let half_size_x = particle_size.x / 2.0;
        let half_size_y = particle_size.y / 2.0;

        let mut new_position = transform.translation.xy() + particle.velocity * delta_time;

        let bounds_x = window_size.width - half_size_x;
        let bounds_y = window_size.height - half_size_y;

        if new_position.x.abs() > bounds_x {
            new_position.x = bounds_x * new_position.x.signum();
            particle.velocity.x *= -bounce_dampening.0;
        }

        if new_position.y.abs() > bounds_y {
            new_position.y = bounds_y * new_position.y.signum();
            particle.velocity.y *= -bounce_dampening.0;

            if particle.velocity.y.abs() < 0.1 {
                particle.velocity.y = 0.0;
            }
        }

        let max_velocity = window_size.width.max(window_size.height) * 2.0;
        particle.velocity = particle.velocity.clamp_length_max(max_velocity);

        transform.translation = new_position.extend(0.0);
    }
}

fn update_grid(
    mut grid: ResMut<Grid>,
    gravity: Res<Gravity>,
    query: Query<(&Transform, &Particle)>,
) {
    let mut previous_velocities = HashMap::new();
    for (cell_idx, cell) in &grid.cells {
        previous_velocities.insert(*cell_idx, cell.velocity);
    }

    grid.clear();

    for (transform, particle) in &query {
        let world_pos = transform.translation.xy();
        let cell_idx = grid.world_to_cell(world_pos);
        let cell_offset = world_pos / grid.cell_size - Vec2::new(cell_idx.0 as f32, cell_idx.1 as f32);

        let weights = [
            0.5 * (0.5 - cell_offset) * (0.5 - cell_offset),
            0.75 - cell_offset * cell_offset,
            0.5 * (0.5 + cell_offset) * (0.5 + cell_offset),
        ];

        for gx in 0..3 {
            for gy in 0..3 {
                let weight = weights[gx].x * weights[gy].y;
                let neighbor_cell = (cell_idx.0 + gx as i32 - 1, cell_idx.1 + gy as i32 - 1);
                let cell = grid.cells.entry(neighbor_cell).or_insert(GridCell::default());

                // Use mass-weighted velocity updates (momentum conservation)
                cell.mass += weight * particle.velocity.length();
                cell.velocity += weight * particle.velocity;
            }
        }
    }
    
    for (cell_idx, cell) in grid.cells.iter_mut() {
        if cell.mass > 0.0 {
            let prev_velocity = previous_velocities.get(cell_idx).copied().unwrap_or(Vec2::ZERO);
            cell.velocity = (cell.velocity + prev_velocity) * 0.5; // Simple velocity smoothing
            cell.velocity += gravity.0; // Apply gravity
        }
    }
}




// Particle Collision Handling
fn resolve_collisions(
    mut query: Query<(Entity, &mut Particle, &mut Transform, &Sprite)>,
) {
    let mut checked_pairs = std::collections::HashSet::<(u32, u32)>::new();
    let mut iter = query.iter_combinations_mut();

    while let Some([
        (entity_a, mut particle_a, mut transform_a, sprite_a),
        (entity_b, mut particle_b, mut transform_b, sprite_b)
    ]) = iter.fetch_next()
    {
        let id_a = entity_a.index();
        let id_b = entity_b.index();

        if id_a == id_b || checked_pairs.contains(&(id_b, id_a)) {
            continue;
        }

        checked_pairs.insert((id_a, id_b));

        let pos_a = transform_a.translation.xy();
        let pos_b = transform_b.translation.xy();
        let radius_a = sprite_a.custom_size.unwrap_or(Vec2::splat(5.0)).x / 2.0;
        let radius_b = sprite_b.custom_size.unwrap_or(Vec2::splat(5.0)).x / 2.0;

        let diff = pos_b - pos_a;
        let distance = diff.length();
        let min_distance = radius_a + radius_b;

        if distance < min_distance {
            let normal = diff.normalize_or_zero();
            let penetration = min_distance - distance;

            // Move particles apart correctly
            let correction = normal * (penetration / 2.0);
            transform_a.translation -= correction.extend(0.0);
            transform_b.translation += correction.extend(0.0);

            // Proper velocity reflection using momentum conservation
            let velocity_a = particle_a.velocity;
            let velocity_b = particle_b.velocity;

            let relative_velocity = velocity_b - velocity_a;
            let velocity_along_normal = relative_velocity.dot(normal);

            if velocity_along_normal > 0.0 {
                continue;
            }

            let restitution = 0.8;
            let impulse_magnitude = -(1.0 + restitution) * velocity_along_normal / 2.0;

            let impulse = normal * impulse_magnitude;
            particle_a.velocity -= impulse;
            particle_b.velocity += impulse;
        }
    }
}
