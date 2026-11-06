import bpy
import hashlib
import json
import math
from pathlib import Path
from mathutils import Vector

ROOT = Path.cwd()
OUTPUT = ROOT / 'target/verification/intact-cave-C-lateral95-export-2026-11-06'
SOURCE = ROOT / 'target/verification/intact-cave-global-core-fit-2026-11-06/global-anisotropic-intact-cave-world.blend'
bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
model = bpy.data.objects['Intact CC0 cave global anisotropic, original474tri']
model.data.calc_loop_triangles()
assert len(model.data.loop_triangles) == 474

# Exactly one authorized whole-source lateral scale change. World X/Y unchanged.
lateral_ratio = 9.5 / 9.0
lateral_changes = []
for vertex in model.data.vertices:
    before = tuple(vertex.co)
    vertex.co.y *= lateral_ratio
    lateral_changes.append({
        'vertex_index': vertex.index,
        'before_blender_world': before,
        'after_blender_world': tuple(vertex.co),
    })
model.data.update()
bpy.ops.wm.save_as_mainfile(filepath=str(OUTPUT / 'whole-source-lateral95-world.blend'))
(OUTPUT / 'whole-source-lateral-change.json').write_text(json.dumps({
    'old_global_scales': [9.0, 2.8, 10.0],
    'new_global_scales': [9.5, 2.8, 10.0],
    'only_world_Z_changed': True,
    'ratio': lateral_ratio,
    'vertices': lateral_changes,
    'no_local_flare_or_other_reshape': True,
}, indent=2))

# Bake only the actual cave geometry. Actors/board cannot imprint AO into its texture.
previous_visibility = {}
for obj in bpy.context.scene.objects:
    if obj.type == 'MESH' and obj != model:
        previous_visibility[obj.name] = obj.hide_render
        obj.hide_render = True
bpy.ops.object.select_all(action='DESELECT')
model.select_set(True)
bpy.context.view_layer.objects.active = model
bpy.ops.object.mode_set(mode='EDIT')
bpy.ops.mesh.select_all(action='SELECT')
bpy.ops.uv.smart_project(angle_limit=1.1, island_margin=0.008)
bpy.ops.object.mode_set(mode='OBJECT')
image = bpy.data.images.new('Original seamless rock and actual cave AO 1K', width=1024, height=1024, alpha=False)
for material in model.data.materials:
    assert material is not None and material.use_nodes
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    shader = nodes.get('Principled BSDF')
    output = nodes.get('Material Output')
    emission = nodes.new('ShaderNodeEmission')
    assert shader.inputs['Base Color'].is_linked
    links.new(shader.inputs['Base Color'].links[0].from_socket, emission.inputs['Color'])
    links.new(emission.outputs[0], output.inputs['Surface'])
    destination = nodes.new('ShaderNodeTexImage')
    destination.image = image
    nodes.active = destination
scene = bpy.context.scene
scene.render.engine = 'CYCLES'
scene.cycles.device = 'CPU'
scene.cycles.samples = 24
scene.render.bake.margin = 8
bpy.ops.object.bake(type='EMIT')
image.filepath_raw = str(OUTPUT / 'cave-stone-and-dark-backing-1k.png')
image.file_format = 'PNG'
image.save()
image.pack()

# Only standard glTF-supported ImageTexture/PBR remains after the bake.
material = bpy.data.materials.new('Standard baked cave stone roughness0.97 dark backing retained')
material.use_nodes = True
shader = material.node_tree.nodes.get('Principled BSDF')
shader.inputs['Metallic'].default_value = 0
shader.inputs['Roughness'].default_value = 0.97
shader.inputs['Alpha'].default_value = 1
texture = material.node_tree.nodes.new('ShaderNodeTexImage')
texture.image = image
material.node_tree.links.new(texture.outputs['Color'], shader.inputs['Base Color'])
model.data.materials.clear()
model.data.materials.append(material)
for face in model.data.polygons:
    face.material_index = 0

# Inverse of unchanged production den parent (15.65,0,0), yaw-.55.
cosine = math.cos(0.55)
sine = math.sin(0.55)
for vertex in model.data.vertices:
    x = vertex.co.x - 15.65
    z = -vertex.co.y
    vertex.co.x = cosine * x + sine * z
    vertex.co.y = -(-sine * x + cosine * z)
model.data.update()
asset = OUTPUT / 'intact_cave_global_C_lateral95_candidate.glb'
bpy.ops.export_scene.gltf(
    filepath=str(asset),
    export_format='GLB',
    use_selection=True,
    export_yup=True,
    export_apply=True,
    export_animations=False,
    export_cameras=False,
    export_lights=False,
)

# Actual exported glTF reimport with the real production parent, not CPU-node preview.
bpy.data.objects.remove(model, do_unlink=True)
for name, hidden in previous_visibility.items():
    bpy.data.objects[name].hide_render = hidden
before = set(bpy.data.objects)
bpy.ops.import_scene.gltf(filepath=str(asset))
imported = [obj for obj in bpy.data.objects if obj not in before]
parent = bpy.data.objects.new('Unchanged production den parent for actual glTF preview', None)
bpy.context.collection.objects.link(parent)
for obj in imported:
    if obj.parent not in imported:
        obj.parent = parent
parent.location = (15.65, 0, 0)
parent.rotation_euler.z = -0.55
camera = scene.camera
coordinates = lambda point: Vector((point[0], -point[2], point[1]))
camera.location = coordinates((-20, 25.2, 10))
camera.rotation_euler = (coordinates((4, 1.2, 0)) - camera.location).to_track_quat('-Z', 'Y').to_euler()
camera.data.ortho_scale = 28 * 1280 / 720
references = [obj for obj in bpy.data.objects if obj.name.startswith('Actual reference enemy_')]
for obj in references:
    obj.location.x = 16.4
scene.render.filepath = str(OUTPUT / 'actual-glTF-C-inside.png')
bpy.ops.render.render(write_still=True)
for obj in references:
    obj.location.x = 12.5
scene.render.filepath = str(OUTPUT / 'actual-glTF-C-after-exit.png')
bpy.ops.render.render(write_still=True)
metadata = {
    'stage': 'Actual exported standard PBR candidate; strong SAT and native pending',
    'asset': str(asset.relative_to(ROOT)),
    'sha256': hashlib.sha256(asset.read_bytes()).hexdigest(),
    'bytes': asset.stat().st_size,
    'source_world_blend_sha256': hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
    'original_474_triangles_retained': True,
    'global_lateral_vertical_depth': [9.5, 2.8, 10.0],
    'source_backing': 'Original six rear backing triangles retained dark, baked with real neck geometry/AO; not light stone fill',
    'bake': '1024x1024 genuine UV ImageTexture; original generated seamless + actual geometry AO; no scan-atlas tiling',
    'image_sha256': hashlib.sha256((OUTPUT / 'cave-stone-and-dark-backing-1k.png').read_bytes()).hexdigest(),
    'actual_reimport_meshes': sum(obj.type == 'MESH' for obj in imported),
    'camera_C': {'eye': [-20, 25.2, 10], 'target': [4, 1.2, 0], 'vertical': 28},
    'original_author': 'OrbitStudios / Fernando Ferreira; submitted by zisongbr',
    'original_license': 'CC0 https://opengameart.org/content/lowpoly-cave-entrance',
    'not_native_or_production_accepted': True,
}
(OUTPUT / 'export-pins.json').write_text(json.dumps(metadata, indent=2))
print('ACTUAL_C_GLTF_EXPORT_REIMPORT_READY', flush=True)
