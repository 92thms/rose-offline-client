use bevy::{prelude::Component, reflect::Reflect};

#[derive(Component, Reflect)]
pub struct ModelHeight {
    pub height: f32,
}

impl ModelHeight {
    pub fn new(height: f32) -> Self {
        Self { height }
    }
}

/// The height name tags should float at while the entity is driving a
/// vehicle, mirroring ModelHeight but computed from the mounted vehicle's
/// body model instead of the pedestrian character model. Present on the
/// driver entity only while `Vehicle` is present and its model has finished
/// loading.
#[derive(Component, Reflect)]
pub struct VehicleModelHeight {
    pub height: f32,
}

impl VehicleModelHeight {
    pub fn new(height: f32) -> Self {
        Self { height }
    }
}
