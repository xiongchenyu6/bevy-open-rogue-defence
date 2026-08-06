"""Generate the project's original low-poly GLB cast with Blender.

Run with: blender --background --python tools/generate_models.py
"""

from math import pi
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets" / "models"
OUT.mkdir(parents=True, exist_ok=True)

HERO_MODEL_NAMES = (
    "hero_storm_savant",
    "hero_ember_ranger",
    "hero_stone_warden",
    "hero_spirit_keeper",
    "hero_tide_walker",
    "hero_star_wagerer",
    "hero_frost_oracle",
    "hero_iron_berserker",
    "hero_rift_dancer",
    "hero_grove_artificer",
    "hero_solar_vanguard",
    "hero_void_reaper",
)

ENEMY_MODEL_NAMES = (
    "enemy_rift_sapper",
    "enemy_gnawling",
    "enemy_shell_guard",
    "enemy_void_caster",
    "enemy_core_rusher",
    "enemy_chanting_acolyte",
    "enemy_iron_colossus",
    "enemy_burrow_stalker",
    "enemy_field_mender",
    "enemy_split_spore",
    "enemy_null_eye",
    "enemy_siege_lobber",
)

BOSS_MODEL_NAMES = (
    "boss_cinder_colossus",
    "boss_abyssal_leviathan",
    "boss_dread_veil",
    "boss_sun_crown_magus",
    "boss_green_regent",
)

SUMMON_MODEL_NAMES = (
    "summon_bone_sentry",
    "summon_spirit_hound",
    "summon_tide_elemental",
    "summon_bolt_turret",
    "summon_frost_wyrm",
    "summon_ancestor_colossus",
)

MODEL_NAME_GROUPS = (
    HERO_MODEL_NAMES,
    ENEMY_MODEL_NAMES,
    BOSS_MODEL_NAMES,
    SUMMON_MODEL_NAMES,
)

EMISSIVE_MATERIAL_TAGS = (
    "acid",
    "bolt",
    "chant",
    "core",
    "crystal",
    "dancer_teal",
    "eye",
    "foam",
    "frost",
    "furnace",
    "glow",
    "iris",
    "payload",
    "rune",
    "serum",
    "singularity",
    "star_gold",
    "tide_pearl",
    "toxin",
    "vanguard_gold",
)

EMISSIVE_MATERIAL_NAMES = {
    "ember_gold",
    "hound_spirit",
    "rusher_ember",
    "sentry_spirit",
    "spirit_glow",
}


def is_emissive_material(name):
    normalized = name.lower()
    return normalized in EMISSIVE_MATERIAL_NAMES or any(
        tag in normalized for tag in EMISSIVE_MATERIAL_TAGS
    )


def reset():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for material in list(bpy.data.materials):
        bpy.data.materials.remove(material)


def material(name, color, metallic=0.0, roughness=0.72):
    value = bpy.data.materials.new(name)
    value.diffuse_color = (*color, 1.0)
    value.use_nodes = True
    shader = value.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*color, 1.0)
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = roughness
    if is_emissive_material(name):
        shader.inputs["Emission Color"].default_value = (
            min(1.0, color[0] * 1.20 + 0.04),
            min(1.0, color[1] * 1.20 + 0.04),
            min(1.0, color[2] * 1.20 + 0.04),
            1.0,
        )
        shader.inputs["Emission Strength"].default_value = 2.2
    return value


def assign(obj, mat):
    obj.data.materials.append(mat)
    return obj


def cube(name, loc, scale, mat, rot=(0.0, 0.0, 0.0), bevel=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc, rotation=rot)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel:
        modifier = obj.modifiers.new("small_bevel", "BEVEL")
        modifier.width = bevel
        modifier.segments = 1
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    return assign(obj, mat)


def sphere(name, loc, scale, mat, subdivisions=1):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=subdivisions, radius=1.0, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return assign(obj, mat)


def cylinder(name, loc, radius, depth, mat, rot=(0.0, 0.0, 0.0), vertices=8):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=loc, rotation=rot
    )
    obj = bpy.context.object
    obj.name = name
    return assign(obj, mat)


def cone(name, loc, r1, r2, depth, mat, rot=(0.0, 0.0, 0.0), vertices=8):
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=r1,
        radius2=r2,
        depth=depth,
        location=loc,
        rotation=rot,
    )
    obj = bpy.context.object
    obj.name = name
    return assign(obj, mat)


def torus(name, loc, major, minor, mat, rot=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_torus_add(
        major_radius=major,
        minor_radius=minor,
        major_segments=12,
        minor_segments=4,
        location=loc,
        rotation=rot,
    )
    obj = bpy.context.object
    obj.name = name
    return assign(obj, mat)


def rod_between(name, start, end, radius, mat, vertices=6):
    start = Vector(start)
    end = Vector(end)
    delta = end - start
    obj = cylinder(name, (start + end) * 0.5, radius, delta.length, mat, vertices=vertices)
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = delta.to_track_quat("Z", "Y")
    return obj


def humanoid_base(primary, accent, dark, heavy=False):
    width = 0.42 if heavy else 0.32
    cylinder("leg_l", (-0.18, 0.0, 0.36), 0.10, 0.72, dark)
    cylinder("leg_r", (0.18, 0.0, 0.36), 0.10, 0.72, dark)
    cone("torso", (0.0, 0.0, 1.12), width, width * 0.72, 0.90, primary)
    sphere("head", (0.0, 0.0, 1.78), (0.27, 0.25, 0.28), accent)
    cone("hood", (0.0, 0.0, 2.02), 0.31, 0.04, 0.42, primary)
    cylinder("arm_l", (-width - 0.12, 0.0, 1.18), 0.08, 0.66, primary, (0.0, 0.28, 0.0))
    cylinder("arm_r", (width + 0.12, 0.0, 1.18), 0.08, 0.66, primary, (0.0, -0.28, 0.0))


def export(name):
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    if len(meshes) > 1:
        bpy.context.view_layer.objects.active = meshes[0]
        bpy.ops.object.join()
    joined = bpy.context.object
    joined.name = name
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)
    # Normalize every cast member to a ground-level pivot. This keeps Bevy
    # spawn transforms consistent even when a design uses floating ornaments.
    min_z = min(vertex.co.z for vertex in joined.data.vertices)
    for vertex in joined.data.vertices:
        vertex.co.z -= min_z
    for polygon in joined.data.polygons:
        polygon.use_smooth = False
    bpy.ops.export_scene.gltf(
        filepath=str(OUT / f"{name}.glb"),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_materials="EXPORT",
    )


def hero_storm():
    reset()
    teal = material("storm_teal", (0.05, 0.54, 0.68), 0.15)
    bronze = material("storm_bronze", (0.48, 0.27, 0.10), 0.55)
    glow = material("storm_crystal", (0.12, 0.85, 1.0), 0.25, 0.28)
    humanoid_base(teal, glow, bronze)
    cube("cape", (0.0, 0.20, 1.18), (0.34, 0.05, 0.62), teal, (0.18, 0.0, 0.0), 0.04)
    torus("bow", (0.62, -0.02, 1.2), 0.47, 0.035, glow, (pi / 2, 0.0, 0.0))
    cube("bow_grip", (0.62, 0.0, 1.2), (0.035, 0.035, 0.48), bronze)
    export("hero_storm_savant")


def hero_ember():
    reset()
    red = material("ember_red", (0.72, 0.10, 0.035), 0.12)
    gold = material("ember_gold", (0.88, 0.52, 0.08), 0.48)
    charcoal = material("ember_dark", (0.08, 0.07, 0.08), 0.25)
    humanoid_base(red, gold, charcoal)
    for side in (-1, 1):
        cube("crossbow", (side * 0.66, -0.02, 1.18), (0.32, 0.08, 0.06), gold, (0.0, 0.0, side * 0.18), 0.025)
        cone("flame", (side * 0.83, 0.0, 1.2), 0.12, 0.0, 0.35, red, (0.0, pi / 2, 0.0))
    export("hero_ember_ranger")


def hero_stone():
    reset()
    steel = material("warden_steel", (0.38, 0.43, 0.48), 0.72)
    stone = material("warden_stone", (0.23, 0.27, 0.25), 0.05)
    cyan = material("warden_rune", (0.10, 0.72, 0.76), 0.15)
    humanoid_base(steel, cyan, stone, heavy=True)
    cylinder("shield", (-0.64, -0.02, 1.18), 0.48, 0.13, steel, (0.0, pi / 2, 0.0), 10)
    cube("shield_rune", (-0.72, -0.02, 1.18), (0.08, 0.25, 0.25), cyan, bevel=0.04)
    cylinder("hammer_handle", (0.64, 0.0, 1.0), 0.055, 1.25, stone)
    cube("hammer_head", (0.64, 0.0, 1.68), (0.34, 0.22, 0.22), steel, bevel=0.05)
    export("hero_stone_warden")


def hero_spirit():
    reset()
    violet = material("spirit_violet", (0.32, 0.10, 0.56), 0.08)
    bone = material("spirit_bone", (0.72, 0.68, 0.54), 0.0)
    green = material("spirit_glow", (0.16, 0.84, 0.46), 0.12)
    humanoid_base(violet, green, bone)
    cylinder("staff", (0.60, 0.0, 1.12), 0.055, 1.9, bone)
    sphere("staff_orb", (0.60, 0.0, 2.12), (0.22, 0.22, 0.22), green)
    for x in (-0.27, 0.27):
        sphere("spirit_orb", (x, 0.18, 2.18), (0.12, 0.12, 0.12), bone)
    export("hero_spirit_keeper")


def hero_tide():
    reset()
    ocean = material("tide_ocean", (0.03, 0.40, 0.54), 0.18)
    pearl = material("tide_pearl", (0.42, 0.86, 0.86), 0.32)
    navy = material("tide_navy", (0.03, 0.10, 0.18), 0.15)
    humanoid_base(ocean, pearl, navy)
    cylinder("trident", (0.62, 0.0, 1.2), 0.045, 2.15, pearl)
    for x in (0.46, 0.62, 0.78):
        cone("trident_tip", (x, 0.0, 2.30), 0.075, 0.0, 0.34, pearl)
    torus("tide_ring", (0.0, 0.1, 0.92), 0.46, 0.045, pearl)
    export("hero_tide_walker")


def hero_star():
    reset()
    indigo = material("star_indigo", (0.12, 0.12, 0.48), 0.16)
    gold = material("star_gold", (0.95, 0.70, 0.12), 0.58)
    white = material("star_white", (0.76, 0.82, 0.92), 0.12)
    humanoid_base(indigo, gold, white)
    for i, x in enumerate((-0.72, -0.48, 0.48, 0.72)):
        cube("fate_card", (x, -0.06, 1.35 + (i % 2) * 0.18), (0.12, 0.025, 0.19), gold, (0.0, 0.12 * x, x * 0.28), 0.02)
    sphere("fate_die", (0.0, -0.12, 2.22), (0.18, 0.18, 0.18), white)
    export("hero_star_wagerer")


def hero_frost_oracle():
    reset()
    ice = material("oracle_ice", (0.30, 0.72, 0.88), 0.22, 0.30)
    crystal = material("oracle_crystal", (0.68, 0.96, 1.0), 0.30, 0.18)
    midnight = material("oracle_midnight", (0.025, 0.08, 0.18), 0.20, 0.58)
    humanoid_base(ice, crystal, midnight)
    cube("frost_mantle", (0.0, 0.20, 1.25), (0.38, 0.06, 0.62), midnight, (0.16, 0.0, 0.0), 0.035)
    cylinder("moon_staff", (0.61, 0.0, 0.86), 0.045, 1.70, midnight, vertices=7)
    crescent_points = (
        (0.60, 0.0, 2.40),
        (0.35, 0.0, 2.34),
        (0.17, 0.0, 2.19),
        (0.12, 0.0, 2.00),
        (0.23, 0.0, 1.82),
        (0.47, 0.0, 1.70),
        (0.61, 0.0, 1.72),
    )
    for start, end in zip(crescent_points, crescent_points[1:]):
        rod_between("moon_crescent", start, end, 0.052, crystal)
    for x, z in ((-0.48, 2.18), (-0.05, 2.38), (0.30, 2.24)):
        cone("floating_ice", (x, 0.16, z + 0.07), 0.085, 0.0, 0.28, crystal, vertices=6)
        cone("floating_ice", (x, 0.16, z - 0.07), 0.085, 0.0, 0.28, crystal, (pi, 0.0, 0.0), 6)
    export("hero_frost_oracle")


def hero_iron_berserker():
    reset()
    iron = material("berserker_iron", (0.055, 0.065, 0.075), 0.82, 0.28)
    crimson = material("berserker_crimson", (0.48, 0.025, 0.025), 0.42, 0.42)
    furnace = material("berserker_furnace", (1.0, 0.25, 0.025), 0.28, 0.18)
    humanoid_base(iron, furnace, crimson, heavy=True)
    for side in (-1, 1):
        cube("iron_pauldron", (side * 0.50, 0.0, 1.52), (0.24, 0.24, 0.18), crimson, bevel=0.055)
        cylinder("axe_handle", (side * 0.70, 0.0, 1.04), 0.055, 1.46, iron, vertices=7)
        cube("axe_head", (side * 0.70, 0.0, 1.75), (0.25, 0.10, 0.11), crimson, bevel=0.025)
        cone("axe_edge", (side * 0.96, 0.0, 1.75), 0.15, 0.015, 0.40, iron, (0.0, side * pi / 2, 0.0), 6)
    sphere("furnace_core", (0.0, -0.35, 1.22), (0.20, 0.10, 0.22), furnace)
    torus("furnace_rim", (0.0, -0.37, 1.22), 0.25, 0.045, crimson, (pi / 2, 0.0, 0.0))
    export("hero_iron_berserker")


def hero_rift_dancer():
    reset()
    magenta = material("dancer_magenta", (0.76, 0.055, 0.58), 0.18, 0.30)
    teal = material("dancer_teal", (0.03, 0.82, 0.68), 0.26, 0.22)
    void = material("dancer_void", (0.035, 0.025, 0.08), 0.30, 0.46)
    humanoid_base(magenta, teal, void)
    for side in (-1, 1):
        cube("split_coat", (side * 0.19, 0.20, 0.83), (0.14, 0.045, 0.48), magenta, (0.18, 0.0, side * 0.18), 0.025)
        torus("moon_blade", (side * 0.68, 0.0, 1.22), 0.31, 0.045, teal, (pi / 2, 0.0, 0.0))
        cone("moon_tip", (side * 0.68, 0.0, 1.60), 0.075, 0.0, 0.28, magenta, vertices=6)
    for x, z, tilt in ((-0.30, 2.22, -0.30), (-0.13, 2.37, -0.78), (0.16, 2.37, 0.78), (0.34, 2.18, 0.30)):
        cube("broken_halo", (x, 0.12, z), (0.13, 0.035, 0.045), teal, (0.0, tilt, 0.0), 0.012)
    sphere("rift_shard", (0.0, -0.10, 2.18), (0.10, 0.07, 0.15), magenta)
    export("hero_rift_dancer")


def hero_grove_artificer():
    reset()
    moss = material("artificer_moss", (0.14, 0.38, 0.16), 0.10, 0.78)
    brass = material("artificer_brass", (0.63, 0.36, 0.07), 0.72, 0.30)
    leaf = material("artificer_leaf_crystal", (0.34, 0.90, 0.42), 0.18, 0.22)
    humanoid_base(moss, leaf, brass)
    cube("turret_pack", (0.0, 0.28, 1.26), (0.33, 0.19, 0.38), brass, bevel=0.045)
    cylinder("turret_pivot", (0.0, 0.30, 1.72), 0.15, 0.34, moss, vertices=8)
    cube("turret_barrel", (-0.25, 0.30, 1.84), (0.38, 0.075, 0.075), brass, (0.0, -0.08, 0.0), 0.018)
    cone("turret_muzzle", (-0.67, 0.30, 1.84), 0.11, 0.055, 0.28, leaf, (0.0, -pi / 2, 0.0), 7)
    cylinder("wrench_staff", (0.66, 0.0, 1.12), 0.05, 1.90, brass, vertices=7)
    cube("wrench_jaw", (0.53, 0.0, 2.06), (0.055, 0.065, 0.19), brass, (0.0, -0.48, 0.0), 0.016)
    cube("wrench_jaw", (0.79, 0.0, 2.06), (0.055, 0.065, 0.19), brass, (0.0, 0.48, 0.0), 0.016)
    cone("leaf_crystal", (0.66, -0.03, 2.23), 0.13, 0.0, 0.36, leaf, vertices=6)
    cone("leaf_crystal", (0.66, -0.03, 2.05), 0.13, 0.0, 0.36, leaf, (pi, 0.0, 0.0), 6)
    for side in (-1, 1):
        cone("leaf_mantle", (side * 0.37, 0.10, 1.62), 0.15, 0.0, 0.46, leaf, (0.0, side * 0.42, 0.0), 6)
    export("hero_grove_artificer")


def hero_solar_vanguard():
    reset()
    ivory = material("vanguard_ivory", (0.78, 0.80, 0.76), 0.62, 0.28)
    gold = material("vanguard_gold", (0.98, 0.62, 0.08), 0.76, 0.20)
    navy = material("vanguard_navy", (0.035, 0.08, 0.16), 0.34, 0.42)
    humanoid_base(ivory, gold, navy, heavy=True)
    cube("sun_breastplate", (0.0, -0.22, 1.26), (0.31, 0.10, 0.38), ivory, bevel=0.055)
    cylinder("sun_shield", (-0.66, -0.02, 1.18), 0.47, 0.14, ivory, (0.0, pi / 2, 0.0), 12)
    torus("sun_wheel", (-0.57, -0.02, 1.18), 0.34, 0.055, gold, (0.0, pi / 2, 0.0))
    sphere("sun_core", (-0.56, -0.10, 1.18), (0.13, 0.08, 0.13), gold)
    for y, z in ((-0.02, 1.61), (-0.02, 0.75), (-0.42, 1.18), (0.38, 1.18)):
        cube("sun_ray", (-0.56, y, z), (0.045, 0.10 if z == 1.18 else 0.045, 0.10 if z != 1.18 else 0.045), gold, bevel=0.012)
    cylinder("short_spear", (0.67, 0.0, 1.12), 0.05, 1.58, navy, vertices=7)
    cone("spear_tip", (0.67, 0.0, 2.02), 0.13, 0.0, 0.44, gold, vertices=7)
    cube("spear_guard", (0.67, 0.0, 1.70), (0.20, 0.055, 0.045), ivory, bevel=0.018)
    export("hero_solar_vanguard")


def hero_void_reaper():
    reset()
    violet = material("reaper_violet", (0.22, 0.025, 0.38), 0.20, 0.42)
    singularity = material("reaper_singularity", (0.78, 0.05, 0.88), 0.34, 0.16)
    black = material("reaper_black", (0.012, 0.012, 0.025), 0.40, 0.36)
    humanoid_base(violet, singularity, black)
    cube("reaper_cloak", (0.0, 0.20, 1.18), (0.38, 0.055, 0.68), black, (0.18, 0.0, 0.0), 0.035)
    cylinder("moon_scythe", (0.68, 0.0, 1.14), 0.05, 2.10, black, vertices=7)
    torus("scythe_crescent", (0.50, 0.0, 2.10), 0.39, 0.060, violet, (pi / 2, 0.0, 0.0))
    sphere("crescent_cut", (0.66, -0.055, 2.10), (0.34, 0.07, 0.34), black)
    cone("scythe_tip", (0.14, 0.0, 2.28), 0.105, 0.0, 0.48, singularity, (0.0, -1.08, 0.0), 6)
    sphere("singularity_orb", (-0.56, -0.02, 1.62), (0.22, 0.22, 0.22), black)
    sphere("singularity_core", (-0.56, -0.18, 1.62), (0.09, 0.06, 0.09), singularity)
    torus("singularity_orbit", (-0.56, -0.02, 1.62), 0.29, 0.025, singularity, (0.50, 0.0, 0.74))
    export("hero_void_reaper")


def enemy_sapper():
    reset()
    rust = material("sapper_rust", (0.46, 0.13, 0.06), 0.52)
    skin = material("sapper_skin", (0.30, 0.48, 0.16), 0.0)
    crystal = material("sapper_crystal", (0.75, 0.08, 0.62), 0.18)
    sphere("body", (0.0, 0.0, 0.58), (0.38, 0.28, 0.45), rust)
    sphere("head", (0.0, -0.02, 1.02), (0.28, 0.24, 0.25), skin)
    cone("helmet", (0.0, 0.0, 1.27), 0.31, 0.04, 0.34, rust)
    cube("pack", (0.0, 0.28, 0.75), (0.25, 0.14, 0.32), crystal, bevel=0.03)
    cylinder("pick_handle", (0.48, 0.0, 0.70), 0.035, 1.15, rust, (0.0, 0.55, 0.0))
    cube("pick_head", (0.66, 0.0, 1.15), (0.30, 0.06, 0.07), crystal, (0.0, 0.0, -0.35), 0.02)
    export("enemy_rift_sapper")


def enemy_gnawling():
    reset()
    hide = material("gnaw_hide", (0.55, 0.20, 0.08), 0.0)
    fang = material("gnaw_fang", (0.78, 0.72, 0.52), 0.0)
    sphere("body", (0.0, 0.0, 0.48), (0.55, 0.28, 0.30), hide)
    sphere("head", (-0.42, -0.02, 0.52), (0.30, 0.25, 0.27), hide)
    for x in (-0.35, 0.30):
        for y in (-0.20, 0.20):
            cylinder("leg", (x, y, 0.20), 0.06, 0.38, hide, (0.18 * y, 0.0, 0.0), 6)
    for y in (-0.13, 0.13):
        cone("fang", (-0.72, y, 0.50), 0.06, 0.0, 0.25, fang, (0.0, -pi / 2, 0.0), 6)
    export("enemy_gnawling")


def enemy_guard():
    reset()
    shell = material("guard_shell", (0.24, 0.18, 0.12), 0.28)
    armor = material("guard_armor", (0.32, 0.38, 0.42), 0.7)
    eye = material("guard_eye", (0.95, 0.24, 0.06), 0.1)
    cube("body", (0.0, 0.0, 0.62), (0.42, 0.30, 0.55), shell, bevel=0.08)
    sphere("head", (0.0, -0.02, 1.18), (0.28, 0.25, 0.25), shell)
    cylinder("shield", (-0.48, -0.02, 0.72), 0.46, 0.16, armor, (0.0, pi / 2, 0.0), 10)
    sphere("eye", (0.0, -0.24, 1.20), (0.08, 0.04, 0.06), eye)
    export("enemy_shell_guard")


def enemy_caster():
    reset()
    robe = material("caster_robe", (0.20, 0.06, 0.36), 0.05)
    green = material("caster_acid", (0.12, 0.78, 0.30), 0.1)
    dark = material("caster_dark", (0.035, 0.025, 0.055), 0.2)
    cone("robe", (0.0, 0.0, 0.62), 0.48, 0.22, 1.20, robe)
    sphere("eye", (0.0, -0.03, 1.42), (0.31, 0.25, 0.29), green)
    torus("halo", (0.0, 0.0, 1.46), 0.44, 0.045, dark, (pi / 2, 0.0, 0.0))
    cylinder("staff", (0.52, 0.0, 0.75), 0.04, 1.45, dark)
    sphere("orb", (0.52, 0.0, 1.52), (0.16, 0.16, 0.16), green)
    export("enemy_void_caster")


def enemy_core_rusher():
    reset()
    hide = material("rusher_hide", (0.34, 0.055, 0.025), 0.05, 0.86)
    iron = material("rusher_iron", (0.22, 0.25, 0.27), 0.78, 0.34)
    horn = material("rusher_horn", (0.80, 0.57, 0.24), 0.20, 0.46)
    ember = material("rusher_ember", (0.95, 0.15, 0.025), 0.18, 0.24)
    sphere("long_body", (0.12, 0.0, 0.58), (0.76, 0.34, 0.38), hide)
    cube("shoulder_ram", (-0.43, 0.0, 0.72), (0.42, 0.42, 0.28), iron, bevel=0.08)
    sphere("armored_head", (-0.78, 0.0, 0.62), (0.38, 0.31, 0.30), iron)
    for y in (-0.22, 0.22):
        cone(
            "ram_horn",
            (-1.12, y, 0.68),
            0.12,
            0.0,
            0.70,
            horn,
            (0.0, -pi / 2, 0.0),
            7,
        )
    for x in (-0.38, 0.42):
        for y in (-0.26, 0.26):
            cylinder("stamp_leg", (x, y, 0.24), 0.09, 0.48, hide, (0.0, 0.0, 0.08 * x), 7)
            cube("hoof", (x - 0.08, y, 0.06), (0.17, 0.11, 0.07), iron, bevel=0.025)
    cone("back_blade", (0.58, 0.0, 0.98), 0.22, 0.0, 0.52, iron, (0.0, 0.25, 0.0), 6)
    sphere("rage_eye", (-0.99, -0.25, 0.72), (0.055, 0.035, 0.055), ember)
    export("enemy_core_rusher")


def enemy_chanting_acolyte():
    reset()
    robe = material("acolyte_robe", (0.22, 0.035, 0.34), 0.05, 0.76)
    brass = material("acolyte_brass", (0.78, 0.43, 0.08), 0.70, 0.28)
    void = material("acolyte_void", (0.018, 0.012, 0.028), 0.16, 0.42)
    chant = material("acolyte_chant", (0.92, 0.18, 0.48), 0.22, 0.20)
    cone("bell_robe", (0.0, 0.0, 0.68), 0.52, 0.20, 1.36, robe)
    sphere("hollow_hood", (0.0, 0.0, 1.50), (0.30, 0.27, 0.31), void)
    cone("spire_hood", (0.0, 0.0, 1.82), 0.34, 0.04, 0.58, robe)
    sphere("chant_face", (0.0, -0.25, 1.52), (0.13, 0.045, 0.10), chant)
    for side in (-1, 1):
        cylinder(
            "raised_arm",
            (side * 0.43, 0.0, 1.36),
            0.075,
            0.82,
            robe,
            (0.0, side * 0.72, 0.0),
            7,
        )
        sphere("ritual_chime", (side * 0.70, 0.0, 1.68), (0.13, 0.13, 0.13), brass)
    cylinder("fork_staff", (0.63, 0.16, 0.92), 0.045, 1.70, brass, vertices=7)
    for x in (0.52, 0.74):
        cylinder("fork_tine", (x, 0.16, 1.80), 0.035, 0.56, brass, vertices=6)
        sphere("chant_note", (x, 0.16, 2.12), (0.09, 0.09, 0.09), chant)
    for z, radius in ((1.34, 0.37), (1.70, 0.48)):
        torus("chant_ring", (0.0, 0.0, z), radius, 0.035, chant, (pi / 2, 0.0, 0.0))
    export("enemy_chanting_acolyte")


def enemy_iron_colossus():
    reset()
    iron = material("colossus_iron", (0.24, 0.28, 0.30), 0.84, 0.32)
    stone = material("colossus_stone", (0.13, 0.16, 0.15), 0.04, 0.92)
    rust = material("colossus_rust", (0.42, 0.12, 0.035), 0.46, 0.58)
    core = material("colossus_core", (0.12, 0.82, 0.70), 0.26, 0.18)
    cube("fortress_torso", (0.0, 0.0, 1.08), (0.64, 0.46, 0.72), iron, bevel=0.10)
    cube("brow_head", (-0.08, 0.0, 1.82), (0.38, 0.34, 0.28), stone, bevel=0.07)
    cube("chest_core", (-0.58, -0.02, 1.14), (0.09, 0.24, 0.27), core, bevel=0.035)
    for side in (-1, 1):
        sphere("boulder_shoulder", (0.0, side * 0.62, 1.40), (0.45, 0.43, 0.45), stone)
        cylinder(
            "piston_arm",
            (0.0, side * 0.74, 0.82),
            0.22,
            0.92,
            iron,
            (0.0, 0.0, side * 0.12),
            8,
        )
        cube("anvil_fist", (-0.08, side * 0.79, 0.34), (0.34, 0.32, 0.28), rust, bevel=0.07)
        cube("slab_foot", (-0.02, side * 0.30, 0.12), (0.37, 0.27, 0.12), iron, bevel=0.04)
    cylinder("left_leg", (0.0, -0.30, 0.44), 0.21, 0.68, stone, vertices=8)
    cylinder("right_leg", (0.0, 0.30, 0.44), 0.21, 0.68, stone, vertices=8)
    for y in (-0.34, 0.0, 0.34):
        cone("back_spike", (0.48, y, 1.68), 0.14, 0.0, 0.48, rust, (0.0, 0.72, 0.0), 6)
    export("enemy_iron_colossus")


def enemy_burrow_stalker():
    reset()
    shell = material("stalker_shell", (0.055, 0.16, 0.24), 0.48, 0.44)
    blade = material("stalker_blade", (0.32, 0.52, 0.56), 0.76, 0.24)
    earth = material("stalker_earth", (0.28, 0.13, 0.055), 0.04, 0.94)
    eye = material("stalker_eye", (1.0, 0.40, 0.04), 0.14, 0.16)
    sphere("burrow_body", (0.10, 0.0, 0.47), (0.68, 0.32, 0.29), shell)
    sphere("drill_head", (-0.55, 0.0, 0.45), (0.36, 0.30, 0.26), earth)
    cone(
        "drill",
        (-0.98, 0.0, 0.44),
        0.27,
        0.0,
        0.88,
        blade,
        (0.0, -pi / 2, 0.0),
        8,
    )
    for index, x in enumerate((-0.30, 0.05, 0.40)):
        for side in (-1, 1):
            cube(
                "digging_leg",
                (x, side * 0.42, 0.26),
                (0.32, 0.07, 0.06),
                blade,
                (0.0, (index - 1) * 0.12, side * (0.36 + index * 0.08)),
                0.025,
            )
    for x, height in ((-0.10, 0.34), (0.28, 0.48), (0.58, 0.30)):
        cone("dorsal_fin", (x, 0.0, 0.78), 0.17, 0.0, height, blade, (0.0, 0.10, 0.0), 6)
    for y in (-0.20, 0.20):
        sphere("ambush_eye", (-0.67, y, 0.56), (0.055, 0.04, 0.055), eye)
    torus("soil_wake", (0.20, 0.0, 0.15), 0.66, 0.055, earth)
    export("enemy_burrow_stalker")


def enemy_field_mender():
    reset()
    shell = material("mender_shell", (0.10, 0.34, 0.24), 0.22, 0.52)
    ivory = material("mender_ivory", (0.72, 0.76, 0.60), 0.10, 0.58)
    serum = material("mender_serum", (0.18, 0.94, 0.50), 0.12, 0.18)
    plum = material("mender_plum", (0.36, 0.08, 0.26), 0.04, 0.74)
    sphere("medic_body", (0.0, 0.0, 0.72), (0.42, 0.34, 0.46), shell)
    cone("flower_mantle", (0.0, 0.0, 1.20), 0.68, 0.22, 0.46, plum, vertices=9)
    sphere("healing_node", (0.0, 0.0, 1.56), (0.25, 0.25, 0.25), serum)
    torus("field_halo", (0.0, 0.0, 1.56), 0.48, 0.055, ivory, (0.32, 0.0, 0.0))
    for side in (-1, 1):
        cylinder("serum_tank", (0.0, side * 0.48, 0.74), 0.15, 0.62, serum, vertices=8)
        torus("tank_guard", (0.0, side * 0.48, 0.74), 0.19, 0.035, ivory)
        for x in (-0.22, 0.24):
            cylinder(
                "spider_leg",
                (x, side * 0.38, 0.30),
                0.055,
                0.68,
                ivory,
                (side * 0.52, x * 0.55, 0.0),
                6,
            )
    cylinder("antenna", (0.20, 0.0, 1.78), 0.035, 0.58, ivory, (0.0, 0.42, 0.0), 6)
    sphere("antenna_orb", (0.32, 0.0, 2.04), (0.10, 0.10, 0.10), serum)
    export("enemy_field_mender")


def enemy_split_spore():
    reset()
    stalk = material("spore_stalk", (0.20, 0.34, 0.16), 0.02, 0.92)
    cap = material("spore_cap", (0.52, 0.06, 0.34), 0.06, 0.70)
    spots = material("spore_spots", (0.96, 0.56, 0.16), 0.16, 0.30)
    toxin = material("spore_toxin", (0.36, 0.92, 0.22), 0.10, 0.22)
    cone("root_stalk", (0.0, 0.0, 0.50), 0.30, 0.18, 0.92, stalk, vertices=8)
    sphere("mother_cap", (0.0, 0.0, 1.22), (0.62, 0.50, 0.32), cap)
    for index, (x, y, z, size) in enumerate(
        (
            (-0.38, -0.28, 0.88, 0.27),
            (0.42, -0.22, 0.82, 0.30),
            (0.25, 0.30, 0.96, 0.24),
        )
    ):
        cylinder("child_stalk", (x, y, z - 0.25), 0.08, 0.48, stalk, vertices=7)
        sphere("child_spore", (x, y, z), (size, size * 0.86, size * 0.72), cap)
        sphere("split_core", (x, y - 0.12, z), (0.08, 0.045, 0.08), toxin)
        cone("cap_spike", (x, y, z + size * 0.72), 0.10, 0.0, 0.25, spots, vertices=6)
    for x, y in ((-0.30, -0.38), (0.02, -0.46), (0.34, -0.36)):
        sphere("cap_spot", (x, y, 1.30), (0.08, 0.04, 0.055), spots)
    torus("spore_sac", (0.0, 0.0, 0.38), 0.34, 0.05, toxin)
    export("enemy_split_spore")


def enemy_null_eye():
    reset()
    sclera = material("null_sclera", (0.68, 0.72, 0.66), 0.08, 0.54)
    iris = material("null_iris", (0.12, 0.68, 0.88), 0.30, 0.20)
    void = material("null_void", (0.012, 0.008, 0.025), 0.44, 0.30)
    frame = material("null_frame", (0.29, 0.10, 0.46), 0.68, 0.26)
    sphere("great_eye", (0.0, 0.0, 1.10), (0.52, 0.48, 0.48), sclera)
    sphere("iris", (-0.47, 0.0, 1.10), (0.08, 0.27, 0.27), iris)
    sphere("pupil", (-0.54, 0.0, 1.10), (0.045, 0.12, 0.12), void)
    torus("outer_orbit", (0.0, 0.0, 1.10), 0.72, 0.07, frame, (0.0, pi / 2, 0.0))
    torus("tilted_orbit", (0.0, 0.0, 1.10), 0.88, 0.045, iris, (0.35, pi / 2, 0.20))
    for index, y in enumerate((-0.38, 0.0, 0.38)):
        cylinder(
            "null_tendril",
            (0.30 + index * 0.10, y, 0.45),
            0.045,
            0.92 + index * 0.12,
            frame,
            (0.0, 0.42 + index * 0.10, 0.0),
            6,
        )
        cone("tendril_tip", (0.50 + index * 0.14, y, 0.04), 0.09, 0.0, 0.26, iris, vertices=6)
    for y in (-0.30, 0.30):
        cone("crown_spike", (0.12, y, 1.72), 0.10, 0.0, 0.44, frame, (0.10, 0.0, y), 6)
    export("enemy_null_eye")


def enemy_siege_lobber():
    reset()
    wood = material("lobber_wood", (0.28, 0.11, 0.035), 0.02, 0.90)
    iron = material("lobber_iron", (0.19, 0.23, 0.25), 0.82, 0.34)
    hide = material("lobber_hide", (0.30, 0.22, 0.08), 0.05, 0.82)
    payload = material("lobber_payload", (0.70, 0.18, 0.04), 0.30, 0.28)
    cube("siege_chassis", (0.0, 0.0, 0.56), (0.72, 0.46, 0.34), wood, bevel=0.06)
    sphere("beast_head", (-0.72, 0.0, 0.55), (0.34, 0.30, 0.29), hide)
    cone("head_plate", (-0.85, 0.0, 0.70), 0.30, 0.10, 0.42, iron, (0.0, -0.45, 0.0), 7)
    for x in (-0.42, 0.42):
        for y in (-0.48, 0.48):
            cylinder("siege_wheel", (x, y, 0.32), 0.28, 0.15, iron, (pi / 2, 0.0, 0.0), 10)
            cylinder("wheel_hub", (x, y * 1.02, 0.32), 0.09, 0.18, payload, (pi / 2, 0.0, 0.0), 8)
    cube("throwing_arm", (0.26, 0.0, 1.28), (0.10, 0.12, 0.90), wood, (0.0, -0.58, 0.0), 0.035)
    cylinder("arm_axle", (-0.04, 0.0, 0.92), 0.15, 1.08, iron, (pi / 2, 0.0, 0.0), 10)
    cube("payload_cup", (0.76, 0.0, 1.92), (0.32, 0.34, 0.12), iron, (0.0, -0.18, 0.0), 0.06)
    sphere("siege_orb", (0.80, 0.0, 2.20), (0.27, 0.27, 0.27), payload)
    for side in (-1, 1):
        cube("brace", (0.10, side * 0.34, 1.00), (0.05, 0.07, 0.64), iron, (0.0, side * 0.40, 0.0), 0.02)
        cone("tusk", (-0.98, side * 0.19, 0.48), 0.07, 0.0, 0.40, payload, (0.0, -pi / 2, 0.0), 6)
    export("enemy_siege_lobber")


def summon_bone_sentry():
    reset()
    bone = material("sentry_bone", (0.72, 0.68, 0.52), 0.04, 0.68)
    iron = material("sentry_iron", (0.18, 0.22, 0.24), 0.78, 0.32)
    spirit = material("sentry_spirit", (0.20, 0.92, 0.56), 0.18, 0.20)
    cylinder("spine", (0.0, 0.0, 0.72), 0.14, 1.10, bone, vertices=7)
    sphere("skull", (0.0, 0.0, 1.38), (0.34, 0.28, 0.30), bone)
    for y in (-0.13, 0.13):
        sphere("eye", (-0.30, y, 1.43), (0.06, 0.045, 0.06), spirit)
    cube("crossbow_stock", (-0.12, 0.0, 1.12), (0.52, 0.08, 0.08), iron, bevel=0.025)
    torus("crossbow_arc", (-0.42, 0.0, 1.14), 0.45, 0.035, bone, (pi / 2, 0.0, 0.0))
    for side in (-1, 1):
        cylinder("rib_leg", (0.12, side * 0.25, 0.28), 0.06, 0.72, bone, (side * 0.40, 0.0, 0.0), 6)
        cone("ground_claw", (0.10, side * 0.40, 0.05), 0.08, 0.0, 0.30, iron, (side * 0.35, 0.0, 0.0), 6)
    export("summon_bone_sentry")


def summon_spirit_hound():
    reset()
    hide = material("hound_hide", (0.12, 0.30, 0.34), 0.08, 0.55)
    spirit = material("hound_spirit", (0.18, 0.92, 0.76), 0.20, 0.18)
    fang = material("hound_fang", (0.82, 0.86, 0.70), 0.08, 0.44)
    sphere("lean_body", (0.12, 0.0, 0.58), (0.72, 0.31, 0.34), hide)
    sphere("hound_head", (-0.58, 0.0, 0.70), (0.36, 0.30, 0.31), hide)
    for y in (-0.17, 0.17):
        cone("fang", (-0.94, y, 0.58), 0.055, 0.0, 0.24, fang, (0.0, -pi / 2, 0.0), 6)
    for x in (-0.32, 0.48):
        for y in (-0.23, 0.23):
            cylinder("runner_leg", (x, y, 0.27), 0.065, 0.54, hide, (0.15 * y, 0.0, 0.0), 6)
    for x in (-0.25, 0.10, 0.44):
        cone("spirit_spine", (x, 0.0, 0.96), 0.11, 0.0, 0.34, spirit, vertices=6)
    cone("spectral_tail", (0.90, 0.0, 0.72), 0.15, 0.0, 0.85, spirit, (0.0, pi / 2, 0.0), 7)
    sphere("spirit_eye", (-0.82, -0.25, 0.78), (0.055, 0.035, 0.055), spirit)
    export("summon_spirit_hound")


def summon_tide_elemental():
    reset()
    water = material("elemental_water", (0.035, 0.42, 0.64), 0.10, 0.25)
    foam = material("elemental_foam", (0.42, 0.90, 0.92), 0.12, 0.18)
    pearl = material("elemental_pearl", (0.72, 0.82, 0.72), 0.56, 0.30)
    sphere("wave_torso", (0.0, 0.0, 0.92), (0.56, 0.42, 0.72), water)
    sphere("tide_core", (-0.38, -0.04, 1.08), (0.15, 0.16, 0.19), foam)
    for side in (-1, 1):
        sphere("wave_shoulder", (0.0, side * 0.48, 1.18), (0.28, 0.30, 0.30), foam)
        cone("wave_arm", (-0.15, side * 0.66, 0.82), 0.19, 0.06, 0.84, water, (side * 0.26, 0.0, 0.0), 7)
    torus("tidal_ring", (0.0, 0.0, 0.38), 0.64, 0.08, foam)
    for angle in (-0.55, 0.0, 0.55):
        cone("crest", (0.35, angle, 1.52), 0.15, 0.0, 0.58, foam, (0.0, 0.52, angle), 6)
    sphere("pearl", (0.28, 0.0, 1.38), (0.16, 0.16, 0.16), pearl)
    export("summon_tide_elemental")


def summon_bolt_turret():
    reset()
    iron = material("turret_iron", (0.20, 0.26, 0.30), 0.84, 0.28)
    copper = material("turret_copper", (0.56, 0.24, 0.06), 0.72, 0.30)
    bolt = material("turret_bolt", (0.12, 0.78, 1.0), 0.26, 0.15)
    cylinder("pivot", (0.0, 0.0, 0.68), 0.28, 0.70, iron, vertices=10)
    for side in (-1, 1):
        cylinder("coil", (0.0, side * 0.32, 1.12), 0.18, 0.58, copper, (pi / 2, 0.0, 0.0), 10)
        torus("coil_ring", (0.0, side * 0.42, 1.12), 0.25, 0.04, bolt, (pi / 2, 0.0, 0.0))
    cube("rail", (-0.24, 0.0, 1.22), (0.64, 0.10, 0.10), iron, bevel=0.025)
    cone("rail_tip", (-0.94, 0.0, 1.22), 0.11, 0.0, 0.44, bolt, (0.0, -pi / 2, 0.0), 7)
    for y in (-0.42, 0.0, 0.42):
        cylinder("tripod", (0.15, y, 0.30), 0.065, 0.76, copper, (y * 0.65, 0.35, 0.0), 7)
    sphere("capacitor", (0.28, 0.0, 1.26), (0.23, 0.23, 0.23), bolt)
    export("summon_bolt_turret")


def summon_frost_wyrm():
    reset()
    scale = material("wyrm_scale", (0.18, 0.42, 0.62), 0.42, 0.34)
    frost = material("wyrm_frost", (0.55, 0.92, 1.0), 0.16, 0.18)
    dark = material("wyrm_dark", (0.05, 0.10, 0.18), 0.58, 0.30)
    sphere("wyrm_body", (0.20, 0.0, 0.92), (0.76, 0.38, 0.38), scale)
    sphere("wyrm_head", (-0.62, 0.0, 1.06), (0.38, 0.30, 0.28), scale)
    cone("snout", (-1.02, 0.0, 1.02), 0.20, 0.04, 0.62, frost, (0.0, -pi / 2, 0.0), 7)
    for side in (-1, 1):
        cube("ice_wing", (0.18, side * 0.58, 1.28), (0.56, 0.08, 0.48), frost, (side * 0.18, 0.0, side * 0.42), 0.04)
        cone("wing_tip", (0.36, side * 1.04, 1.30), 0.13, 0.0, 0.68, dark, (side * 0.72, 0.0, 0.0), 6)
    for x in (-0.15, 0.18, 0.48):
        cone("ice_spine", (x, 0.0, 1.30), 0.10, 0.0, 0.36, frost, vertices=6)
    cone("tail", (0.92, 0.0, 0.88), 0.22, 0.0, 1.12, scale, (0.0, pi / 2, 0.0), 8)
    sphere("frost_eye", (-0.84, -0.23, 1.14), (0.05, 0.035, 0.05), frost)
    for x in (-0.20, 0.48):
        for y in (-0.22, 0.22):
            cylinder("claw_leg", (x, y, 0.50), 0.06, 0.54, dark, (0.12 * y, 0.0, 0.0), 6)
    export("summon_frost_wyrm")


def summon_ancestor_colossus():
    reset()
    stone = material("ancestor_stone", (0.30, 0.34, 0.32), 0.06, 0.90)
    moss = material("ancestor_moss", (0.12, 0.42, 0.20), 0.04, 0.84)
    rune = material("ancestor_rune", (0.10, 0.86, 0.72), 0.20, 0.20)
    cube("monolith_torso", (0.0, 0.0, 1.30), (0.78, 0.52, 0.84), stone, bevel=0.12)
    cube("ancestor_head", (-0.10, 0.0, 2.22), (0.44, 0.38, 0.36), moss, bevel=0.08)
    cube("heart_rune", (-0.70, -0.02, 1.42), (0.10, 0.30, 0.30), rune, bevel=0.035)
    for side in (-1, 1):
        sphere("stone_shoulder", (0.0, side * 0.72, 1.62), (0.48, 0.46, 0.48), stone)
        cylinder("pillar_arm", (-0.10, side * 0.82, 0.94), 0.25, 1.12, moss, (0.0, 0.0, side * 0.08), 8)
        cube("stone_fist", (-0.18, side * 0.86, 0.34), (0.38, 0.34, 0.30), stone, bevel=0.07)
        cylinder("column_leg", (0.05, side * 0.30, 0.48), 0.25, 0.82, stone, vertices=8)
        cube("slab_foot", (-0.16, side * 0.30, 0.10), (0.40, 0.30, 0.11), moss, bevel=0.04)
    for y in (-0.38, 0.0, 0.38):
        cone("ancestral_spire", (0.58, y, 1.92), 0.15, 0.0, 0.54, rune, (0.0, 0.62, 0.0), 6)
    export("summon_ancestor_colossus")


def validate_models(names, label):
    """Re-import generated GLBs and enforce the engine-facing contract."""
    print(f"{label} GLB validation (single static mesh, grounded pivot):")
    for name in names:
        reset()
        path = OUT / f"{name}.glb"
        bpy.ops.import_scene.gltf(filepath=str(path))
        meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
        armatures = [obj for obj in bpy.context.scene.objects if obj.type == "ARMATURE"]
        if len(meshes) != 1:
            raise RuntimeError(f"{name}: expected 1 mesh, found {len(meshes)}")
        if armatures:
            raise RuntimeError(f"{name}: expected static mesh, found armature")
        obj = meshes[0]
        world_min_z = min((obj.matrix_world @ vertex.co).z for vertex in obj.data.vertices)
        materials = [slot.material for slot in obj.material_slots if slot.material is not None]
        material_count = len(materials)
        emissive_count = 0
        if abs(world_min_z) > 1e-5:
            raise RuntimeError(f"{name}: world min-Z is {world_min_z:.8f}, expected 0")
        if abs(obj.matrix_world.translation.z) > 1e-5:
            raise RuntimeError(f"{name}: pivot Z is not grounded")
        if material_count < 2:
            raise RuntimeError(f"{name}: expected at least 2 PBR materials")
        if obj.animation_data is not None or bpy.data.actions:
            raise RuntimeError(f"{name}: expected a static mesh without animation")
        for pbr_material in materials:
            principled = (
                pbr_material.node_tree.nodes.get("Principled BSDF")
                if pbr_material.use_nodes and pbr_material.node_tree
                else None
            )
            if principled is None:
                raise RuntimeError(f"{name}: material {pbr_material.name} is not PBR")
            emission_strength = principled.inputs["Emission Strength"].default_value
            if emission_strength > 0.0:
                emissive_count += 1
            if (
                is_emissive_material(pbr_material.name)
                and emission_strength <= 0.0
            ):
                raise RuntimeError(
                    f"{name}: tagged material {pbr_material.name} is not emissive"
                )
        print(
            f"  {path.name}: vertices={len(obj.data.vertices)} "
            f"materials={material_count} emissive={emissive_count} "
            f"min_z={world_min_z:.6f}"
        )


def boss_model(name, primary_color, accent_color, style):
    reset()
    primary = material(f"{name}_body", primary_color, 0.28)
    accent = material(f"{name}_core", accent_color, 0.42, 0.3)
    dark = material(f"{name}_dark", (0.04, 0.045, 0.055), 0.55)
    if style == 0:
        cube("torso", (0.0, 0.0, 1.5), (0.88, 0.58, 1.1), primary, bevel=0.16)
        sphere("core", (0.0, -0.52, 1.62), (0.34, 0.12, 0.40), accent)
        for x in (-0.92, 0.92):
            cylinder("arm", (x, 0.0, 1.45), 0.28, 1.4, primary, (0.0, x * 0.22, 0.0))
            cone("horn", (x * 0.62, 0.0, 2.80), 0.20, 0.0, 0.85, accent, (0.0, -x * 0.32, 0.0))
    elif style == 1:
        sphere("shell", (0.0, 0.0, 1.10), (1.35, 0.82, 0.72), primary)
        sphere("head", (-1.02, -0.03, 1.20), (0.55, 0.50, 0.48), accent)
        for i in range(6):
            x = -0.75 + i * 0.30
            cylinder("tentacle", (x, 0.28 if i % 2 else -0.28, 0.45), 0.12, 1.10, dark, (0.25, 0.0, (i - 2.5) * 0.12), 7)
        torus("shell_ring", (0.0, 0.0, 1.2), 1.05, 0.16, accent)
    elif style == 2:
        cone("robe", (0.0, 0.0, 1.25), 1.05, 0.34, 2.5, primary)
        sphere("mask", (0.0, -0.05, 2.62), (0.42, 0.30, 0.50), accent)
        for side in (-1, 1):
            cube("wing", (side * 1.05, 0.18, 1.7), (0.72, 0.10, 1.0), dark, (0.0, side * 0.25, side * 0.35), 0.05)
    elif style == 3:
        cone("body", (0.0, 0.0, 1.35), 0.82, 0.30, 2.4, primary)
        sphere("sun_core", (0.0, 0.0, 2.65), (0.48, 0.48, 0.48), accent)
        for radius, tilt in ((0.72, 0.0), (1.02, 0.55), (1.28, -0.42)):
            torus("solar_ring", (0.0, 0.0, 2.65), radius, 0.07, accent, (tilt, 0.0, tilt * 0.6))
    else:
        cone("mantle", (0.0, 0.0, 1.25), 1.10, 0.38, 2.4, primary)
        sphere("regent_head", (0.0, 0.0, 2.75), (0.48, 0.40, 0.50), accent)
        for side in (-1, 1):
            cylinder("antler", (side * 0.46, 0.0, 3.35), 0.09, 1.25, dark, (0.0, side * 0.42, 0.0), 7)
            cone("blade", (side * 1.05, 0.0, 1.35), 0.26, 0.0, 1.45, accent, (0.0, side * 0.65, 0.0), 7)
        torus("crown", (0.0, 0.0, 3.10), 0.62, 0.08, accent)
    export(name)


hero_storm()
hero_ember()
hero_stone()
hero_spirit()
hero_tide()
hero_star()
hero_frost_oracle()
hero_iron_berserker()
hero_rift_dancer()
hero_grove_artificer()
hero_solar_vanguard()
hero_void_reaper()
enemy_sapper()
enemy_gnawling()
enemy_guard()
enemy_caster()
enemy_core_rusher()
enemy_chanting_acolyte()
enemy_iron_colossus()
enemy_burrow_stalker()
enemy_field_mender()
enemy_split_spore()
enemy_null_eye()
enemy_siege_lobber()
boss_model("boss_cinder_colossus", (0.52, 0.06, 0.02), (1.0, 0.35, 0.03), 0)
boss_model("boss_abyssal_leviathan", (0.03, 0.24, 0.42), (0.08, 0.72, 0.96), 1)
boss_model("boss_dread_veil", (0.26, 0.04, 0.40), (0.72, 0.12, 0.92), 2)
boss_model("boss_sun_crown_magus", (0.62, 0.20, 0.02), (1.0, 0.72, 0.10), 3)
boss_model("boss_green_regent", (0.02, 0.34, 0.14), (0.08, 0.92, 0.38), 4)
summon_bone_sentry()
summon_spirit_hound()
summon_tide_elemental()
summon_bolt_turret()
summon_frost_wyrm()
summon_ancestor_colossus()
model_count = sum(len(names) for names in MODEL_NAME_GROUPS)
print(f"Generated {model_count} GLB models in {OUT}")
validate_models(HERO_MODEL_NAMES, "Hero")
validate_models(ENEMY_MODEL_NAMES, "Enemy")
validate_models(BOSS_MODEL_NAMES, "Boss")
validate_models(SUMMON_MODEL_NAMES, "Summon")
