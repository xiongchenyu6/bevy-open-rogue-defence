"""Blender contact sheet: blender -b -t 3 --python tools/render_models.py -- assets/models screenshots/model-cast-review.png"""

import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete(use_global=False)
files = sorted(
    p for p in Path(sys.argv[-2]).glob("*.glb") if not p.name.startswith("environment_")
)
columns = 7
rows = math.ceil(len(files) / columns)
for i, p in enumerate(files):
    before = set(bpy.context.scene.objects)
    old_actions = set(bpy.data.actions)
    bpy.ops.import_scene.gltf(filepath=str(p))
    objs = set(bpy.context.scene.objects) - before
    for o in objs:
        if o.parent is None:
            o.matrix_world = Matrix.Rotation(math.pi, 4, "Z") @ o.matrix_world
            o.location.x += (i % columns) * 5
            o.location.y += (i // columns) * 6
    for o in objs:
        if o.type == "ARMATURE" and o.animation_data:
            for t in o.animation_data.nla_tracks:
                t.mute = True
            actions = [
                a
                for a in set(bpy.data.actions) - old_actions
                if a.name.startswith("Idle")
            ]
            if actions:
                o.animation_data.action = actions[-1]
                if actions[-1].slots:
                    o.animation_data.action_slot = actions[-1].slots[0]
    bpy.ops.object.text_add(
        location=((i % columns) * 5 - 2, (i // columns) * 6 - 2, 0.02)
    )
    text = bpy.context.object
    text.data.body = p.stem.replace("hero_", "").replace("_", " ")
    text.data.size = 0.22
    ink = bpy.data.materials.get("label") or bpy.data.materials.new("label")
    ink.diffuse_color = (0.6, 0.8, 0.95, 1)
    ink.use_nodes = True
    ink.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (
        0.6,
        0.8,
        0.95,
        1,
    )
    text.data.materials.append(ink)
bpy.context.scene.frame_set(8)
bpy.ops.mesh.primitive_plane_add(size=200, location=(0, 0, -0.05))
mat = bpy.data.materials.new("floor")
mat.diffuse_color = (0.025, 0.035, 0.045, 1)
mat.use_nodes = True
mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (
    0.025,
    0.035,
    0.045,
    1,
)
bpy.context.object.data.materials.append(mat)
bpy.ops.object.camera_add(location=(15, -18, 37))
camera = bpy.context.object
camera.rotation_euler = (
    (Vector((15, 12, 0.8)) - camera.location).to_track_quat("-Z", "Y").to_euler()
)
camera.data.type = "ORTHO"
camera.data.ortho_scale = 37
bpy.context.scene.camera = camera
for loc, power, size in [((10, -4, 25), 9000, 15), ((20, 22, 20), 7000, 12)]:
    bpy.ops.object.light_add(type="AREA", location=loc)
    bpy.context.object.data.energy = power
    bpy.context.object.data.shape = "DISK"
    bpy.context.object.data.size = size
    bpy.context.object.rotation_euler = (
        (Vector((15, 12, 0)) - bpy.context.object.location)
        .to_track_quat("-Z", "Y")
        .to_euler()
    )
s = bpy.context.scene
s.render.engine = "CYCLES"
s.cycles.samples = 16
s.render.resolution_x = 1920
s.render.resolution_y = 1600
s.render.resolution_percentage = 100
s.world.color = (0.3, 0.3, 0.3)
s.render.filepath = sys.argv[-1]
bpy.ops.render.render(write_still=True)
