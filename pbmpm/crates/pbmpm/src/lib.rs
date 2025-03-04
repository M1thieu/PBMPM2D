pub mod particle;
pub mod grid;

use bevy::prelude::*;
use glam::Vec2;

#[derive(Resource)]
pub struct Gravity(pub Vec2);

#[derive(Resource)]
pub struct BounceDampening(pub f32);
