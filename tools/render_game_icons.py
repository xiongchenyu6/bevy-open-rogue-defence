"""Render game emblems: blender -b -t 4 --python tools/render_game_icons.py.

Optional canonical icon ids after -- select a subset. Requires ImageMagick.
"""
import json
import math
import re
import subprocess
import sys
from itertools import pairwise
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/ui/icons'
OUT.mkdir(parents=True, exist_ok=True)
COLORS = {
    'gold': (0.95, .57, .12, 1), 'silver': (.64, .82, .92, 1),
    'storm': (.12, .64, 1, 1), 'fire': (1, .19, .025, 1),
    'ice': (.18, .91, 1, 1), 'tide': (.035, .47, .9, 1),
    'spirit': (.20, .88, .44, 1), 'arcane': (.62, .16, .92, 1),
    'blood': (.72, .035, .10, 1), 'dark': (.018, .027, .05, 1),
    'wood': (.36, .15, .045, 1), 'bone': (.91, .85, .62, 1),
}


def material(name):
    existing = bpy.data.materials.get(name)
    if existing:
        return existing
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = COLORS[name]
    mat.use_nodes = True
    node = mat.node_tree.nodes['Principled BSDF']
    node.inputs['Base Color'].default_value = COLORS[name]
    node.inputs['Metallic'].default_value = .62 if name in ('gold', 'silver') else .20
    node.inputs['Roughness'].default_value = .28
    return mat


def finish(obj, color):
    obj.data.materials.append(material(color))
    bevel = obj.modifiers.new('Rounded forged edges', 'BEVEL')
    bevel.width = .045
    bevel.segments = 2
    obj.modifiers.new('Weighted highlights', 'WEIGHTED_NORMAL')
    return obj


def poly(points, color='gold', z=0, thickness=.22):
    n = len(points)
    vertices = [(x, y, z + dz) for dz in (0, thickness) for x, y in points]
    faces = [tuple(reversed(range(n))), tuple(range(n, 2*n))]
    faces += [(i, (i+1) % n, (i+1) % n+n, i+n) for i in range(n)]
    mesh = bpy.data.meshes.new('Forged silhouette')
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new('Emblem', mesh)
    bpy.context.collection.objects.link(obj)
    return finish(obj, color)


def rod(a, b, radius=.10, color='gold', z=.10):
    av, bv = Vector((*a, z)), Vector((*b, z))
    bpy.ops.mesh.primitive_cylinder_add(vertices=12, radius=radius, depth=(bv-av).length, location=(av+bv)/2)
    obj = bpy.context.object
    obj.rotation_euler = (bv-av).to_track_quat('Z', 'Y').to_euler()
    return finish(obj, color)


def gem(x=0, y=0, size=.35, color='arcane', z=.35):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=1, radius=size, location=(x, y, z))
    obj = bpy.context.object
    obj.scale.z = .65
    return finish(obj, color)


def ring(radius=.65, color='gold', z=.05, start=0, end=360):
    angles = [math.radians(start+(end-start)*i/36) for i in range(37)]
    for a, b in pairwise(angles):
        rod((radius*math.cos(a), radius*math.sin(a)), (radius*math.cos(b), radius*math.sin(b)), .075, color, z)


def bolt(x=0, y=0, size=1, color='storm'):
    return poly([(x+a*size, y+b*size) for a,b in [(.18,1),(-.62,-.08),(-.12,-.08),(-.28,-1),(.63,.27),(.12,.27)]],color,.12)


def blade(x=0, y=0, size=1, color='silver', angle=0):
    before = set(bpy.context.scene.objects)
    poly([(-.16,-.35),(-.19,.64),(0,1.12),(.19,.64),(.16,-.35)],color,.12)
    rod((-.4,-.37),(.4,-.37),.09,'gold',.16)
    rod((0,-.38),(0,-.91),.11,'wood',.1)
    gem(0,-.94,.15,'gold')
    bpy.context.view_layer.update()
    for obj in set(bpy.context.scene.objects)-before:
        obj.matrix_world = Matrix.Translation((x,y,0)) @ Matrix.Rotation(angle,4,'Z') @ Matrix.Diagonal((size,size,size,1)) @ obj.matrix_world


def flame(color='fire'):
    poly([(-.62,-.35),(-.45,.4),(-.19,.04),(.10,1.05),(.4,.53),(.42,.08),(.63,.41),(.73,-.29),(.4,-.72),(-.16,-.82)],color)
    poly([(-.25,-.36),(.1,.34),(.16,-.1),(.4,-.3),(.2,-.62),(-.1,-.61)],'gold',.26,.16)


def shield(color='storm'):
    shape=[(-.8,.65),(0,.95),(.8,.65),(.65,-.36),(0,-1),(-.65,-.36)]
    poly(shape,'gold')
    poly([(x*.78,y*.78) for x,y in shape],color,.22,.16)
    gem(0,.06,.29,'silver',.5)


def skull():
    poly([(-.7,.45),(-.45,.9),(.45,.9),(.7,.45),(.62,-.22),(.38,-.35),(.35,-.8),(-.35,-.8),(-.38,-.35),(-.62,-.22)],'bone')
    for x in (-.3,.3):
        gem(x,.16,.24,'dark',.32)
    poly([(-.11,-.19),(.11,-.19),(0,-.4)],'dark',.31,.05)
    for x in (-.2,0,.2):
        rod((x,-.62),(x,-.85),.035,'dark',.27)


def star(color='arcane', count=5):
    points=[]
    for i in range(count*2):
        a=math.pi/2+math.pi*i/count
        r=.95 if i%2==0 else .44
        points.append((math.cos(a)*r,math.sin(a)*r))
    poly(points,'gold')
    poly([(x*.72,y*.72) for x,y in points],color,.23,.18)
    gem(0,0,.22,'silver',.48)


def leaf():
    poly([(-.65,-.62),(-.78,.03),(-.45,.57),(.8,1),(.68,-.02),(.3,-.57)],'spirit')
    rod((-.78,-.9),(.48,.62),.065,'gold',.3)
    for x,y in [(-.32,-.35),(-.08,-.05),(.17,.26)]:
        rod((x,y),(x+.42,y-.10),.035,'gold',.3)


def snow():
    for i in range(6):
        a=i*math.pi/3
        end=(.94*math.cos(a),.94*math.sin(a))
        rod((0,0),end,.065,'ice')
        for s in (-1,1):
            base=(.6*math.cos(a),.6*math.sin(a))
            tip=(base[0]+.28*math.cos(a+s*.9),base[1]+.28*math.sin(a+s*.9))
            rod(base,tip,.05,'silver')
    gem(0,0,.27,'ice')


def arrow(x=0, y=0, color='gold', angle=-.5):
    before=set(bpy.context.scene.objects)
    rod((0,-.8),(0,.63),.075,color)
    poly([(-.3,.38),(0,1),(.3,.38)],color)
    poly([(-.25,-.9),(0,-.68),(.25,-.9),(.23,-.46),(0,-.27),(-.23,-.46)],'silver')
    bpy.context.view_layer.update()
    for obj in set(bpy.context.scene.objects)-before:
        obj.matrix_world=Matrix.Translation((x,y,0)) @ Matrix.Rotation(angle,4,'Z') @ obj.matrix_world


def book():
    poly([(-.85,-.58),(-.85,.72),(-.1,.6),(0,.44),(.1,.6),(.85,.72),(.85,-.58),(0,-.79)],'gold')
    for s in (-1,1):
        poly([(s*.08,-.53),(s*.08,.4),(s*.70,.52),(s*.70,-.45)],'arcane',.24,.14)
    gem(0,0,.27,'ice',.48)


def crown():
    poly([(-.8,-.5),(-1,.65),(-.45,.25),(0,1),(.45,.25),(1,.65),(.8,-.5)],'gold')
    rod((-.75,-.45),(.75,-.45),.13,'gold',.3)
    for x in (-.5,0,.5):
        gem(x,-.15,.18,'arcane' if x else 'fire',.35)


def wave(color='tide'):
    for i in range(3):
        pts=[(-.85,-.65+i*.5),(-.5,-.52+i*.5),(0,-.66+i*.5),(.5,-.38+i*.5),(.9,-.4+i*.5),(.6,-.69+i*.5),(0,-.86+i*.5),(-.5,-.73+i*.5)]
        poly(pts,color,z=i*.055)


def coins():
    for x,y,z in [(-.35,-.3,0),(.35,-.25,.1),(0,.35,.2)]:
        bpy.ops.mesh.primitive_cylinder_add(vertices=12,radius=.48,depth=.18,location=(x,y,z))
        finish(bpy.context.object,'gold')
        gem(x,y,.17,'gold',z+.16)


def refresh():
    ring(.7,'spirit',start=35,end=315)
    poly([(.78,.72),(.25,.69),(.73,.16)],'gold',.2)


def symbol(kind,color='arcane'):
    {'bolt':lambda:bolt(color=color),'flame':lambda:flame(color),'shield':lambda:shield(color),'skull':skull,'star':lambda:star(color), 'leaf':leaf,'snow':snow,'blade':lambda:blade(color=color),'wave':lambda:wave(color),'book':book,'crown':crown,'coins':coins,'refresh':refresh}[kind]()


# Each icon is a distinct sculpted silhouette or authored summon model.
DESIGNS={
 'cultivation':('cauldron','gold','seal'), 'spirit_energy':('gems','spirit','triune'),
 'element_master':('gems','gold','elements'), 'element_archmage':('book','arcane','matrix'),
 'demon_hunter':('arrows','blood','cross'), 'barbarian':('blades','blood','wings'),
 'mad_scientist':('flask','arcane',''), 'windrunner':('arrow','storm','wings'),
 'forbidden_arts':('book','blood','cross'), 'revival_wind':('leaf','spirit','orbit'),
 'mountain':('shield','gold','matrix'), 'killing_intent':('blade','blood','rays'),
 'destruction':('meteor','fire','matrix'), 'requiem':('skull','arcane','orbit'),
 'hunter':('arrow','wood','cross'), 'sharpshooter':('arrow','gold','matrix'),
 'flame_ray':('ray','fire',''), 'goblin_mine':('mine','wood',''),
 'arc_chain':('bolt','storm','chain'),'ember_arrow':('arrow','fire',''),
 'frost_lance':('blade','ice','snow'),'tidal_fan':('wave','tide','fan'),
 'orbit_blade':('blade','silver','orbit'),'piercing_volley':('arrows','gold',''),
 'arcane_star':('star','arcane',''),'ground_pulse':('fist','gold',''),
 'static_seal':('bolt','storm','seal'),'cinder_aura':('flame','fire','orbit'),
 'storm_matrix':('bolt','storm','matrix'),'meteor_seed':('meteor','fire',''),
 'glacier_core':('snow','ice',''),'vortex_column':('wave','tide','vortex'),
 'execution_brand':('skull','blood','cross'),'flying_swords':('blades','silver',''),
 'prism_echo':('gems','arcane',''),'oath_banner':('banner','gold',''),
 'transmute_spark':('coins','gold','spark'),'blood_wheel':('blades','blood','wheel'),
 'sky_judgment':('blade','gold','rays'),'ember_rebirth':('flame','fire','wings'),
 'triune_engine':('gems','arcane','triune'),'fate_cube':('cube','arcane',''),
 'storm_conduit':('bolt','storm','wings'),'ember_oath':('flame','fire','cross'),
 'spirit_pact':('leaf','spirit','wings'),'element_cycle':('gems','tide','elements'),
 'arcane_loop':('star','arcane','orbit'),'triune_union':('gems','gold','triune'),
 'qi_ascension':('arrow','spirit','rays'),'blood_array':('skull','blood','matrix'),
 'raider_fortune':('coins','gold','crown'),'guardian_oath':('shield','storm','wings'),
 'gold':('coins','gold',''),'wood':('wood','wood',''),'skull':('skull','bone',''),
 'shield':('shield','storm',''),'energy':('bolt','gold',''),'tower':('tower','silver',''),
 'skill':('book','arcane',''),'bond':('links','gold',''),'market':('chest','gold',''),
 'dodge':('arrow','ice','speed'),'ultimate':('crown','gold','rays'),
 'refresh':('refresh','spirit',''),'skip':('arrows','silver',''),
 'later':('hourglass','gold',''),'confirm':('check','spirit',''),'abandon':('cross','blood',''),
}
SUMMONS=['bone_sentry','spirit_hound','tide_elemental','bolt_turret','frost_wyrm','ancestor_colossus']


def sculpt(name):
    kind,color,accent=DESIGNS[name]
    if accent in ('orbit','seal','wheel'):
        ring(.95,'gold' if accent!='wheel' else 'blood',z=-.1)
    if accent in ('wings','rays'):
        for side in (-1,1):
            for i in range(3):
                poly([(side*.45,-.4+i*.24),(side*1.06,.0+i*.32),(side*.94,-.35+i*.23)],'gold',-.12)
    if accent=='matrix':
        for x in (-.7,.7):
            for y in (-.7,.7):
                gem(x,y,.22,color)
                rod((x,y),(0,0),.04,'gold',-.1)
    if accent in ('cross','chain'):
        for x in (-.75,.75):
            gem(x,-.65,.2,color)
    if kind == 'wave' and accent == 'vortex':
        for i in range(5):
            width = .24 + i * .16
            y = -.85 + i * .35
            poly([(-width,y+.18),(width,y+.26),(width*.78,y-.02),(-width*.7,y-.12)], 'tide' if i % 2 else 'ice', z=i*.06)
    elif kind in ('bolt','flame','shield','skull','star','leaf','snow','blade','wave','book','crown','coins','refresh'):
        symbol(kind,color)
    elif kind=='cauldron':
        bpy.ops.mesh.primitive_cone_add(vertices=24, radius1=.46, radius2=.65, depth=.65)
        finish(bpy.context.object,'gold')
        bpy.ops.mesh.primitive_cylinder_add(vertices=24, radius=.54, depth=.025, location=(0,0,.34))
        finish(bpy.context.object,'dark')
        ring(.62,'gold',z=.35)
        for n in range(3):
            angle=n*math.tau/3
            bpy.ops.mesh.primitive_cylinder_add(vertices=12, radius=.10, depth=.55, location=(.4*math.cos(angle),.4*math.sin(angle),-.53))
            finish(bpy.context.object,'gold')
        for x in (-.72,.72):
            bpy.ops.mesh.primitive_torus_add(major_radius=.18, minor_radius=.045, major_segments=16, minor_segments=6, location=(x,0,.27), rotation=(math.pi/2,0,0))
            finish(bpy.context.object,'gold')
        gem(0,0,.24,'spirit',.56)
    elif kind=='flask':
        poly([(-.25,1),(.25,1),(.25,.2),(.8,-.6),(.65,-.85),(-.65,-.85),(-.8,-.6),(-.25,.2)],'silver')
        poly([(-.53,-.32),(.53,-.32),(.65,-.67),(-.65,-.67)],'spirit',.3)
        for x,y in [(-.15,.2),(.2,.5),(-.3,.65)]:
            gem(x,y,.14,'arcane',.4)
    elif kind=='ray':
        gem(-.75,-.65,.3,'fire')
        for i in (-1,0,1):
            rod((-.62,-.53+i*.15),(.8,.6+i*.15),.11,'fire' if i else 'gold',.2)
        star('gold')
    elif kind=='mine':
        bpy.ops.mesh.primitive_uv_sphere_add(segments=16, ring_count=8, radius=.65, location=(0,0,.1))
        bpy.context.object.scale.z=.45
        finish(bpy.context.object,'wood')
        ring(.6,'silver',z=.13)
        gem(0,0,.25,'fire',.45)
        for a in range(6):
            angle=a*math.tau/6
            rod((.45*math.cos(angle),.45*math.sin(angle)),(.95*math.cos(angle),.95*math.sin(angle)),.1,'silver',.1)
    elif kind=='arrow':
        arrow(color=color)
    elif kind=='arrows':
        for x,y in [(-.5,-.18),(0,.12),(.5,-.18)]:
            arrow(x,y,color,angle=-.25)
    elif kind=='blades':
        for a in (-.65,0,.65):
            blade(size=.76,color=color,angle=a)
    elif kind=='gems':
        colors=['fire','ice','storm','spirit'] if accent=='elements' else [color,'ice','fire']
        for i,c in enumerate(colors):
            a=math.pi/2+2*math.pi*i/len(colors)
            gem(math.cos(a)*.56,math.sin(a)*.56,.43,c)
        if accent=='triune':
            ring(.72,'gold',z=-.05)
    elif kind=='cube':
        bpy.ops.mesh.primitive_cube_add(size=1.25,rotation=(.45,.4,.3))
        finish(bpy.context.object,'arcane')
        for x,y in [(-.28,-.28),(.28,-.28),(-.28,.28),(.28,.28)]:
            gem(x,y,.11,'gold',.88)
    elif kind=='meteor':
        gem(-.25,-.3,.62,'fire')
        for i in range(3):
            poly([(-.2+i*.23,.0),(.1+i*.25,.8+i*.14),(.4+i*.22,.18)],'gold',-.05)
    elif kind=='banner':
        rod((-.6,-1),(-.6,1),.09,'gold')
        poly([(-.5,.85),(.8,.85),(.8,-.35),(.18,-.64),(-.5,-.35)],'storm')
        gem(.12,.15,.31,'gold')
    elif kind in ('tower','chest'):
        poly([(-.7,-.8),(.7,-.8),(.6,.62),(-.6,.62)],'silver' if kind=='tower' else 'wood')
        if kind=='tower':
            for x in (-.55,0,.55):
                poly([(x-.16,.42),(x-.16,.96),(x+.16,.96),(x+.16,.42)],'gold')
            poly([(-.17,-.8),(-.17,-.1),(.17,-.1),(.17,-.8)],'dark',.25)
        else:
            rod((-.66,.2),(.66,.2),.11,'gold',.3)
            for x in (-.5,.5):
                rod((x,-.72),(x,.57),.07,'gold',.3)
            gem(0,.08,.22,'gold')
    elif kind=='wood':
        for x,y in [(-.34,-.3),(.34,-.3),(0,.38)]:
            rod((x-.27,y-.34),(x+.27,y+.34),.26,'wood')
            gem(x+.27,y+.34,.24,'bone',.22)
    elif kind=='links':
        for x in (-.35,.35):
            before=set(bpy.context.scene.objects)
            ring(.48,'gold' if x<0 else 'silver')
            for o in set(bpy.context.scene.objects)-before:
                o.location.x+=x
    elif kind=='hourglass':
        poly([(-.62,.8),(.62,.8),(.16,0),(.62,-.8),(-.62,-.8),(-.16,0)],'ice')
        for y in (-.86,.86):
            rod((-.75,y),(.75,y),.12,'gold')
        gem(0,-.45,.25,'gold')
    elif kind=='check':
        poly([(-.9,-.08),(-.64,.2),(-.18,-.25),(.71,.8),(1,.52),(-.15,-.8)],'spirit')
    elif kind=='cross':
        for a in (-1,1):
            rod((-.7,-.7*a),(.7,.7*a),.17,'blood')
    elif kind=='fist':
        poly([(-.62,-.65),(-.75,.3),(-.5,.53),(-.32,.3),(-.1,.63),(.13,.4),(.35,.58),(.65,.24),(.5,-.7)],'gold')
        for x in (-.36,-.05,.26):
            rod((x,.3),(x,-.18),.06,'dark',.25)
    if accent=='spark':
        star('ice',4)
    if accent=='crown':
        gem(0,.94,.22,'arcane')
    if accent=='speed':
        for y in (-.5,0,.5):
            rod((-.95,y),(-.5,y),.065,'gold')


def render(name):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene=bpy.context.scene
    if name in SUMMONS:
        bpy.ops.import_scene.gltf(filepath=str(ROOT/f'assets/models/summon_{name}.glb'))
        for obj in list(scene.objects):
            if obj.parent is None:
                obj.matrix_world=Matrix.Rotation(math.pi,4,'Z') @ obj.matrix_world
            if obj.type=='ARMATURE' and obj.animation_data:
                for track in obj.animation_data.nla_tracks:
                    track.mute=True
                idle=next((a for a in bpy.data.actions if a.name.startswith('Idle')),None)
                if idle:
                    obj.animation_data.action=idle
                    if idle.slots:
                        obj.animation_data.action_slot=idle.slots[0]
        scene.frame_set(8)
        direction=Vector((.36,-1,.28))
    else:
        sculpt(name)
        # Preserve the diagonal X for the destructive action at tiny HUD sizes.
        direction=Vector((.4,-.85,.65)) if name == 'cultivation' else Vector((0,0,1)) if name == 'abandon' else Vector((.12,-.18,1))
    bpy.context.view_layer.update()
    meshes=[o for o in scene.objects if o.type=='MESH']
    points=[o.matrix_world@Vector(v) for o in meshes for v in o.bound_box]
    low=Vector(tuple(min(v[i] for v in points) for i in range(3)))
    high=Vector(tuple(max(v[i] for v in points) for i in range(3)))
    target=(low+high)/2
    span=max(high-low)
    bpy.ops.object.camera_add(location=target+direction.normalized()*span*5)
    camera=bpy.context.object
    camera.rotation_euler=(target-camera.location).to_track_quat('-Z','Y').to_euler()
    camera.data.type='ORTHO'
    camera.data.ortho_scale=span*1.55
    scene.camera=camera
    for offset,power,color in [((-3,-4,6),800,(1,.88,.66)),((4,1,4),650,(.55,.76,1)),((-2,3,2),900,(.4,.75,1))]:
        bpy.ops.object.light_add(type='AREA',location=target+Vector(offset)*span/2)
        light=bpy.context.object
        light.data.energy=power*(span/2)**2
        light.data.size=3*span/2
        light.data.color=color
        light.rotation_euler=(target-light.location).to_track_quat('-Z','Y').to_euler()
    scene.world=bpy.data.worlds.new('Icon studio')
    scene.world.use_nodes=True
    scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.18,.23,.32,1)
    scene.world.node_tree.nodes['Background'].inputs[1].default_value=.45
    scene.render.engine='CYCLES'
    scene.cycles.samples=20
    scene.cycles.use_denoising=True
    scene.render.film_transparent=True
    scene.render.resolution_x=256
    scene.render.resolution_y=256
    scene.render.resolution_percentage=100
    scene.render.image_settings.file_format='PNG'
    scene.render.image_settings.color_mode='RGBA'
    scene.view_settings.view_transform='AgX'
    scene.view_settings.exposure=-.45
    output=OUT/f'{name}.png'
    scene.render.filepath=str(output)
    bpy.ops.render.render(write_still=True)
    subprocess.run(['magick',str(output),'-trim','+repage','-resize','228x228','-background','none','-gravity','center','-extent','256x256',str(output)],check=True)
    print(f'ICON_DONE {name}',flush=True)


content=(ROOT/'src/game/content.rs').read_text()
skills=re.findall(r'id: "([a-z_]+)"',content.split('pub const SKILLS:')[1].split('pub const ENEMIES:')[0])
# Reusable visual motifs; gameplay cards and families map to these assets below.
bonds=['cultivation', 'spirit_energy', 'element_master', 'element_archmage', 'demon_hunter', 'barbarian', 'mad_scientist', 'windrunner', 'forbidden_arts', 'revival_wind', 'mountain', 'killing_intent', 'destruction', 'requiem', 'hunter', 'sharpshooter', 'storm_conduit', 'ember_oath', 'spirit_pact', 'element_cycle', 'arcane_loop', 'triune_union', 'qi_ascension', 'blood_array', 'raider_fortune', 'guardian_oath']
reference=(ROOT/'src/game/bond_catalog.rs').read_text()
expanded = (ROOT / 'src/game/bond_catalog/expanded.rs').read_text()
card_source = reference.split('const BASE_CARDS:')[1].split('const BASE_FAMILIES:')[0] + expanded.split('const CARDS:')[1].split('const FAMILIES:')[0]
family_source = reference.split('const BASE_FAMILIES:')[1].split('const fn joined')[0] + expanded.split('const FAMILIES:')[1]
def icon_mapping(source):
    return dict(re.findall(r'id:\s*"([a-z_]+)".*?icon:\s*"([a-z_]+)"',source,re.S))
assert len(skills)==32
assert set(skills+bonds).issubset(set(DESIGNS)|set(SUMMONS))
all_names=list(DESIGNS)+SUMMONS
selected=set(sys.argv[sys.argv.index('--')+1:]) if '--' in sys.argv else set()
for icon in all_names:
    if not selected or icon in selected:
        render(icon)
manifest={'bond_cards':icon_mapping(card_source),'bond_families':icon_mapping(family_source),'skills':skills,'bonds':bonds,'hud':[n for n in DESIGNS if n not in skills+bonds], 'size':[256,256], 'format':'RGBA PNG', 'summon_model_icons':SUMMONS}
(OUT/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
