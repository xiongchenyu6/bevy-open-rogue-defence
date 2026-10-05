"""Render the recruitment hall: blender -b -t 4 --python tools/render_draft_backdrop.py."""
import math
from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[1]
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)
scene = bpy.context.scene
collections = {}


def prop(name, position, height, yaw=0):
    if name not in collections:
        before = set(scene.objects)
        bpy.ops.import_scene.gltf(filepath=str(ROOT / f'assets/models/environment_{name}.glb'))
        collection = bpy.data.collections.new(name)
        for obj in set(scene.objects) - before:
            for old in list(obj.users_collection):
                old.objects.unlink(obj)
            collection.objects.link(obj)
        collections[name] = collection
    instance = bpy.data.objects.new(name, None)
    instance.instance_type = 'COLLECTION'
    instance.instance_collection = collections[name]
    scene.collection.objects.link(instance)
    instance.location = position
    instance.scale = (height,) * 3
    instance.rotation_euler.z = yaw


def area(name, position, target, power, color, size):
    bpy.ops.object.light_add(type='AREA', location=position)
    obj = bpy.context.object
    obj.name = name
    obj.data.energy = power
    obj.data.color = color
    obj.data.shape = 'DISK'
    obj.data.size = size
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat('-Z', 'Y').to_euler()


# Vanishing-point composition frames portrait cards with authored stonework.
for side in (-1, 1):
    for depth, h in [(-1, 9.5), (6, 8.5), (12, 7.5), (18, 7)]:
        prop('pillar_decorated', (side * 8.2, depth, 0), h, side * math.pi / 2)
        prop('wall_window_closed', (side * 10.2, depth + 2.5, 0), 7, math.pi / 2)
    for depth in (1, 10, 17):
        prop('torch_lit', (side * 7.0, depth, 2.4), 1.8)
        bpy.ops.object.light_add(type='POINT', location=(side * 6.8, depth - 0.3, 3.9))
        bpy.context.object.data.energy = 390
        bpy.context.object.data.color = (1.0, 0.43, 0.13)
        bpy.context.object.data.shadow_soft_size = 1.2
    prop('rubble_large', (side * 8.8, -3.0, 0), 1.6, side * 0.5)
    prop('rubble_half', (side * 6.8, 17, 0), 1.5, side * 0.8)
    prop('banner_blue', (side * 7.2, 17, 3.4), 3.5)
    prop('wall_broken', (side * 5.5, 22, 0), 7.5)
# Dim stone aisle catches only gentle pools of reflected light.
for row in range(11):
    for col in range(-3, 4):
        prop('floor', (col * 2.0, row * 2.0 - 4, -0.15), 0.15)
bpy.ops.mesh.primitive_plane_add(size=200, location=(0, 0, -0.18))
mat = bpy.data.materials.new('Midnight foundation')
mat.use_nodes = True
bsdf = mat.node_tree.nodes['Principled BSDF']
bsdf.inputs['Base Color'].default_value = (0.006, 0.015, 0.022, 1)
bsdf.inputs['Roughness'].default_value = 0.72
bpy.context.object.data.materials.append(mat)
area('Cold vault light', (0, 10, 13), (0, 10, 0), 1050, (0.14, 0.48, 0.62), 12)
area('Blue rim left', (-11, 3, 8), (-6, 5, 4), 850, (0.12, 0.43, 0.62), 7)
area('Blue rim right', (11, 3, 8), (6, 5, 4), 850, (0.12, 0.43, 0.62), 7)
area('Distant sanctuary', (0, 23, 10), (0, 8, 4), 950, (0.11, 0.46, 0.53), 6)
# A modest volume softens depth while leaving silhouettes visible at screen edges.
bpy.ops.mesh.primitive_cube_add(size=1, location=(0, 10, 7))
fog = bpy.context.object
fog.name = 'Temple haze'
fog.scale = (34, 52, 20)
volume = bpy.data.materials.new('Teal atmospheric dust')
volume.use_nodes = True
nodes = volume.node_tree.nodes
nodes.clear()
out = nodes.new('ShaderNodeOutputMaterial')
v = nodes.new('ShaderNodeVolumePrincipled')
v.inputs['Density'].default_value = 0.009
v.inputs['Color'].default_value = (0.11, 0.22, 0.27, 1)
v.inputs['Anisotropy'].default_value = 0.25
volume.node_tree.links.new(v.outputs['Volume'], out.inputs['Volume'])
fog.data.materials.append(volume)
bpy.ops.object.camera_add(location=(0, -15, 5.0))
camera = bpy.context.object
camera.rotation_euler = (Vector((0, 14, 4.7)) - camera.location).to_track_quat('-Z', 'Y').to_euler()
camera.data.lens = 26
scene.camera = camera
scene.render.engine = 'CYCLES'
scene.cycles.samples = 40
scene.cycles.use_denoising = True
scene.render.resolution_x = 1600
scene.render.resolution_y = 1000
scene.render.resolution_percentage = 100
scene.world.use_nodes = True
scene.world.node_tree.nodes['Background'].inputs[0].default_value = (0.014, 0.028, 0.045, 1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value = 0.16
scene.view_settings.view_transform = 'AgX'
scene.view_settings.exposure = -0.5
scene.render.image_settings.file_format = 'PNG'
scene.render.filepath = str(ROOT / 'assets/ui/draft_backdrop.png')
bpy.ops.render.render(write_still=True)
print('DRAFT_BACKDROP_RENDER_COMPLETE', scene.render.filepath)
