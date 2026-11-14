//! Shared, texture-free skill shading adapted from LinearAbiltyCastingThreeJS.
//! See THIRD_PARTY_NOTICES/LinearAbilityCasting.txt for upstream licensing.
use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(super) struct SkillMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    /// x is motif, y is the pause-aware presentation clock.
    #[uniform(0)]
    pub parameters: Vec4,
}
impl Material for SkillMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/skill_energy.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/skill_energy.wgsl".into()
    }
    fn enable_shadows() -> bool {
        false
    }
    fn enable_prepass() -> bool {
        false
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
