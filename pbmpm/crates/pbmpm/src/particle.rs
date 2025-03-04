use bevy::prelude::*;
use glam::Vec2;

#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
}

pub fn update_particles(
    mut query: Query<(&mut Particle, &mut Transform, &Sprite)>,
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

        if new_position.x.abs() > 400.0 - half_size_x {
            new_position.x = (400.0 - half_size_x) * new_position.x.signum();
            particle.velocity.x *= -bounce_dampening.0;
        }

        if new_position.y.abs() > 300.0 - half_size_y {
            new_position.y = (300.0 - half_size_y) * new_position.y.signum();
            particle.velocity.y *= -bounce_dampening.0;

            if particle.velocity.y.abs() < 0.1 {
                particle.velocity.y = 0.0;
            }
        }

        transform.translation = new_position.extend(0.0);
    }
}
