use bevy::{
    math::Vec3,
    prelude::{Commands, DespawnRecursiveExt, Entity, Handle, Mesh, Query, With, Without},
    render::primitives::Aabb,
};

use rose_data::VehiclePartIndex;

use crate::components::{ModelHeight, NameTagEntity, Vehicle, VehicleModel, VehicleModelHeight};

/// Computes the height of a mounted vehicle's body model, so name tags can
/// float above the vehicle instead of at pedestrian height while driving.
/// Mirrors the AABB approach used for character/NPC ModelHeight.
pub fn vehicle_model_height_system(
    mut commands: Commands,
    query_add: Query<(Entity, &VehicleModel), Without<ModelHeight>>,
    query_aabb: Query<Option<&Aabb>, With<Handle<Mesh>>>,
    query_vehicle_owners: Query<(Entity, &Vehicle, Option<&NameTagEntity>)>,
) {
    for (entity, vehicle_model) in query_add.iter() {
        let mut min = None;
        let mut max = None;
        let mut all_parts_loaded = true;

        for part_entity in vehicle_model.model_parts[VehiclePartIndex::Body].1.iter() {
            match query_aabb.get(*part_entity) {
                Ok(Some(aabb)) => {
                    min = Some(min.map_or_else(|| aabb.min(), |min: bevy::math::Vec3A| {
                        min.min(aabb.min())
                    }));
                    max = Some(max.map_or_else(|| aabb.max(), |max: bevy::math::Vec3A| {
                        max.max(aabb.max())
                    }));
                }
                Ok(None) => {
                    all_parts_loaded = false;
                    break;
                }
                Err(_) => {
                    all_parts_loaded = false;
                    break;
                }
            }
        }

        if !all_parts_loaded {
            // Body model parts not yet loaded, try again next frame
            continue;
        }

        let height = if let (Some(min), Some(max)) = (min, max) {
            let min = Vec3::from(min);
            let max = Vec3::from(max);
            0.65 + (max.y - min.y)
        } else {
            // No body parts at all (shouldn't normally happen), fall back to
            // a conservative default rather than looping forever.
            2.5
        };
        commands.entity(entity).insert(ModelHeight::new(height));

        // Store the resolved height directly on the driver entity, and
        // despawn its name tag (spawned using the pedestrian ModelHeight)
        // so it gets recreated at the correct vehicle height.
        if let Some((driver_entity, _, name_tag_entity)) = query_vehicle_owners
            .iter()
            .find(|(_, vehicle, _)| vehicle.vehicle_model_entity == entity)
        {
            commands
                .entity(driver_entity)
                .insert(VehicleModelHeight::new(height));

            if let Some(name_tag_entity) = name_tag_entity {
                commands.entity(driver_entity).remove::<NameTagEntity>();
                commands.entity(name_tag_entity.0).despawn_recursive();
            }
        }
    }
}
