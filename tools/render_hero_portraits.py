"""Render authored hero portraits: blender -b -t 4 --python tools/render_hero_portraits.py.

Transparent 384x480 PNGs preserve each model's real materials and idle pose.
Optional hero filename stems after -- limit the render to selected heroes.
"""
import math
import subprocess
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets/ui/portraits"
OUTPUT.mkdir(parents=True, exist_ok=True)
selected = set(sys.argv[sys.argv.index("--") + 1:]) if "--" in sys.argv else set()


def aim(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()


def render(path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    bpy.ops.import_scene.gltf(filepath=str(path))
    models = list(scene.objects)
    for obj in models:
        if obj.parent is None:
            obj.matrix_world = Matrix.Rotation(math.pi, 4, "Z") @ obj.matrix_world
        if obj.type == "ARMATURE" and obj.animation_data:
            for track in obj.animation_data.nla_tracks:
                track.mute = True
            idle = next((a for a in bpy.data.actions if a.name.startswith("Idle")), None)
            if idle:
                obj.animation_data.action = idle
                if idle.slots:
                    obj.animation_data.action_slot = idle.slots[0]
    scene.frame_set(8)
    bpy.context.view_layer.update()
    depsgraph = bpy.context.evaluated_depsgraph_get()
    points = []
    for obj in models:
        if obj.type == "MESH" and not obj.hide_render:
            evaluated = obj.evaluated_get(depsgraph)
            mesh = evaluated.to_mesh()
            points.extend(evaluated.matrix_world @ v.co for v in mesh.vertices)
            evaluated.to_mesh_clear()
    low = Vector(tuple(min(v[i] for v in points) for i in range(3)))
    high = Vector(tuple(max(v[i] for v in points) for i in range(3)))
    target = (low + high) * 0.5
    height = high.z - low.z
    bpy.ops.object.camera_add(location=target + Vector((0.34, -1.0, 0.22)) * height * 4)
    camera = bpy.context.object
    aim(camera, target)
    camera.data.type = "ORTHO"
    bpy.context.view_layer.update()
    inv = camera.matrix_world.inverted()
    projected = [inv @ p for p in points]
    min_x, max_x = min(p.x for p in projected), max(p.x for p in projected)
    min_y, max_y = min(p.y for p in projected), max(p.y for p in projected)
    camera.location += camera.rotation_euler.to_matrix() @ Vector(((min_x + max_x) / 2, (min_y + max_y) / 2, 0))
    camera.data.ortho_scale = max(max_y - min_y, (max_x - min_x) / 0.8) * 1.12
    scene.camera = camera
    for name, offset, energy, color, size in [
        ("Warm softbox", (-2.4, -3.4, 4.5), 650, (1.0, 0.86, 0.72), 3.5),
        ("Cool fill", (3.0, -2.0, 2.0), 350, (0.64, 0.79, 1.0), 3.0),
        ("Silver rim", (1.0, 2.0, 3.5), 1000, (0.73, 0.87, 1.0), 2.5),
    ]:
        bpy.ops.object.light_add(type="AREA", location=target + Vector(offset) * height / 2)
        light = bpy.context.object
        light.name = name
        light.data.energy = energy * (height / 2) ** 2
        light.data.color = color
        light.data.shape = "DISK"
        light.data.size = size * height / 2
        aim(light, target)
    scene.world = bpy.data.worlds.new("Portrait studio")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.16, 0.20, 0.30, 1)
    scene.world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.45
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 32
    scene.cycles.use_denoising = True
    scene.render.film_transparent = True
    scene.render.resolution_x = 384
    scene.render.resolution_y = 480
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.view_settings.view_transform = "AgX"
    scene.render.filepath = str(OUTPUT / (path.stem + ".png"))
    bpy.ops.render.render(write_still=True)
    # Fit the rendered alpha silhouette, not unused/occluded rig geometry.
    # A shared bottom margin keeps feet grounded across the draft roster.
    subprocess.run([
        "magick", scene.render.filepath, "-trim", "+repage", "-resize", "344x440",
        "-background", "none", "-gravity", "south", "-extent", "384x460",
        "-gravity", "north", "-extent", "384x480", scene.render.filepath,
    ], check=True)
    print(f"PORTRAIT_DONE {path.stem}", flush=True)


for source in sorted((ROOT / "assets/models").glob("hero_*.glb")):
    if not selected or source.stem in selected:
        render(source)
