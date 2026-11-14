// Procedural hash and hot-core / soft-sheath layering ported from
// achrefelouafi/LinearAbiltyCastingThreeJS (MIT, 2026 mohamedachrefelouafi).
// Sources: shaders/lib/noise.glsl.js, materials/BeamMaterial.js,
// materials/LightningMaterial.js. Attribution accompanies the distribution.
#import bevy_pbr::{mesh_functions, forward_io::Vertex, view_transformations::position_world_to_clip, mesh_view_bindings::view}

struct SkillMaterial { color: vec4<f32>, parameters: vec4<f32> }
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: SkillMaterial;
struct EnergyVertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) local: vec3<f32>,
    @location(1) world: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) @interpolate(flat) life: f32,
    @location(4) color: vec4<f32>,
}
@vertex fn vertex(input: Vertex) -> EnergyVertex {
    var out: EnergyVertex;
    let model = mesh_functions::get_world_from_local(input.instance_index);
    let world = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(input.position, 1.0));
    out.clip = position_world_to_clip(world.xyz);
    out.world = world.xyz;
    out.local = input.position;
    out.normal = mesh_functions::mesh_normal_local_to_world(input.normal, input.instance_index);
    out.life = f32(mesh_functions::get_tag(input.instance_index)) / 65535.0;
#ifdef VERTEX_COLORS
    out.color = input.color;
#else
    out.color = vec4<f32>(1.0);
#endif
    return out;
}
fn hash13(point: vec3<f32>) -> f32 {
    var p = fract(point * 0.1031);
    p += vec3<f32>(dot(p, p.zyx + vec3<f32>(31.32)));
    return fract((p.x + p.y) * p.z);
}
fn noise(point: vec3<f32>) -> f32 {
    let i = floor(point);
    let f = fract(point);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(mix(hash13(i), hash13(i + vec3<f32>(1.,0.,0.)), u.x),
                   mix(hash13(i + vec3<f32>(0.,1.,0.)), hash13(i + vec3<f32>(1.,1.,0.)), u.x), u.y),
               mix(mix(hash13(i + vec3<f32>(0.,0.,1.)), hash13(i + vec3<f32>(1.,0.,1.)), u.x),
                   mix(hash13(i + vec3<f32>(0.,1.,1.)), hash13(i + vec3<f32>(1.,1.,1.)), u.x), u.y), u.z);
}
@fragment fn fragment(input: EnergyVertex) -> @location(0) vec4<f32> {
    let time = material.parameters.y;
    let motif = u32(material.parameters.x);
    let p = input.local;
    let n = noise(p * 5.0 + vec3<f32>(0., -time * 2.1, time * 0.7));
    let fine = noise(p * 13.0 - vec3<f32>(time * 0.8));
    let turbulence = n * 0.72 + fine * 0.28;
    let facing = abs(dot(normalize(input.normal), normalize(view.world_position - input.world)));
    let sheath = pow(clamp(facing, 0.0, 1.0), 0.45);
    let core = smoothstep(0.40, 0.83, turbulence);
    var tint = material.color.rgb;
    var brightness = 1.1 + core * 2.5;
    var opacity = sheath * (0.30 + turbulence * 0.65);
    if motif == 0u {
        // Ember tongues rise through an amber body to a pale hot core.
        tint = mix(tint * 0.55, vec3<f32>(1.0, 0.75, 0.20), core);
        opacity *= smoothstep(0.12, 0.42, turbulence);
    } else if motif == 1u || motif == 3u {
        // Crystal/water faces remain translucent, with moving internal veins.
        let vein = pow(1.0 - abs(turbulence * 2.0 - 1.0), 8.0);
        brightness = 0.65 + vein * 2.0 + pow(1.0 - facing, 3.0);
        opacity = 0.34 + sheath * 0.40;
    } else if motif == 2u {
        brightness = 1.3 + core * 3.0;
        opacity *= 0.78 + 0.22 * sin(time * 37.0 + p.y * 19.0);
    } else if motif == 4u {
        // Living toxin has dark gaps and bright moving membranes.
        brightness = 0.6 + core * 1.8;
        opacity *= smoothstep(0.22, 0.53, turbulence);
    }
    let fade = smoothstep(0.0, 0.28, input.life);
    let hot = mix(tint, vec3<f32>(1.0), core * 0.24);
    return vec4<f32>(hot * brightness * input.color.rgb, opacity * fade * input.color.a);
}
