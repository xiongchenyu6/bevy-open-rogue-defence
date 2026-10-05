//! Matte world finishes. Imported art keeps its textures and animated geometry;
//! luminous combat effects and UI materials keep their own rendering settings.
use bevy::prelude::*;

pub(super) struct MaterialFinishPlugin;

impl Plugin for MaterialFinishPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, finish_imported_materials);
    }
}

fn finish_imported_materials(
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // A GLTF subasset's Added event can arrive before its asset path is
    // registered. Retry unresolved subassets, including late-loaded camps.
    // Only collect unfinished materials; avoid GPU uploads every frame.
    let pending: Vec<_> = materials
        .iter()
        .filter_map(|(id, material)| {
            let path = server.get_path(id)?;
            let path = path.path().to_string_lossy();
            let (roughness, reflectance) = if material.metallic > 0.5 {
                (0.58, 0.36)
            } else {
                (0.88, 0.24)
            };
            (path.starts_with("models/")
                && path.ends_with(".glb")
                && !material.unlit
                && (material.perceptual_roughness < roughness
                    || material.reflectance != reflectance
                    || material.clearcoat != 0.0))
                .then_some(id)
        })
        .collect();
    for id in pending {
        let Some(mut material) = materials.get_mut(id) else {
            continue;
        };
        if material.unlit {
            continue;
        }
        // The cast textures combine cloth, skin and equipment in one atlas.
        // Broad, weak highlights suit those atlases; actual metallic scenery
        // keeps a narrower highlight instead of becoming chalky stone.
        if material.metallic > 0.5 {
            material.perceptual_roughness = material.perceptual_roughness.max(0.58);
            material.reflectance = 0.36;
        } else {
            material.perceptual_roughness = material.perceptual_roughness.max(0.88);
            material.reflectance = 0.24;
        }
        material.clearcoat = 0.0;
    }
}
