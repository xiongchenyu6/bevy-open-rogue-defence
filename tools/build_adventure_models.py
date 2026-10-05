"""Original procedural camp creatures: blender -b -t 3 --python tools/build_adventure_models.py."""
import bpy, math, json, struct, hashlib
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/models'
COLORS={
'embermaw':[(.16,.095,.075),(.34,.16,.07),(.9,.31,.065),(.92,.78,.5)],
 'tidehorn':[(.12,.25,.26),(.22,.43,.43),(.15,.65,.73),(.65,.82,.74)],
 'plaguejaw':[(.18,.24,.13),(.31,.17,.37),(.49,.64,.15),(.79,.7,.46)]}
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
for datablocks in (bpy.data.actions,bpy.data.armatures,bpy.data.meshes):
    for d in list(datablocks):
        if d.users==0:datablocks.remove(d)
all_roots=[];manifest=[]

def material(name,color,metal=0,rough=.88,glow=0):
 m=bpy.data.materials.new(name);m.diffuse_color=(*color,1);m.use_nodes=True
 p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(*color,1);p.inputs['Roughness'].default_value=rough;p.inputs['Metallic'].default_value=metal
 if glow:p.inputs['Emission Color'].default_value=(*color,1);p.inputs['Emission Strength'].default_value=glow
 return m

def build(kind,index):
 parts=[];bones={'root':((0,0,0),(0,0,.3),None),'body':((0,0,.35),(0,0,.8),'root'),'head':((0,.4,.62),(0,.95,.62),'body'),'tail':((0,-.5,.6),(0,-1.05,.65),'body')}
 mats=[material(kind+'_'+str(i),c,rough=.92 if i==0 else .83,glow=.3 if i==2 and kind=='embermaw' else 0) for i,c in enumerate(COLORS[kind])]
 eye=material(kind+'_eyes',(1,.55,.08) if index==0 else ((.22,.88,1) if index==1 else (.78,1,.12)),rough=.7,glow=1)
 def part(name,pos,scale,mat,bone='body',cone=False,direction=None):
  if cone:bpy.ops.mesh.primitive_cone_add(vertices=6,radius1=1,radius2=0,depth=2,location=pos)
  else:bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2 if name=='body' else 1,radius=1,location=pos)
  o=bpy.context.object;o.name=kind+'_'+name;o.scale=scale
  if direction:o.rotation_euler=Vector(direction).to_track_quat('Z','Y').to_euler()
  bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
  o.data.materials.append(mat);g=o.vertex_groups.new(name=bone);g.add(list(range(len(o.data.vertices))),1,'REPLACE');parts.append(o);return o
 def link(name,a,b,width,mat,bone):
  midpoint=(Vector(a)+Vector(b))/2;length=(Vector(b)-Vector(a)).length
  return part(name,midpoint,(width,width,length/2),mat,bone,direction=Vector(b)-Vector(a))
 width=[.36,.56,.43][index];length=[.64,.63,.73][index]
 part('body',(0,0,.58),(width,length,.34),mats[0])
 part('shoulders',(0,.36,.65),(width*1.06,.35,.34),mats[1])
 if index==0:
  part('skull',(0,.72,.64),(.26,.35,.26),mats[0],'head')
  part('jaw',(0,.91,.5),(.26,.29,.12),mats[1],'head')
  part('snout',(0,1.04,.62),(.2,.16,.15),mats[0],'head')
  for side in [-1,1]:
   part('eye',(side*.205,.91,.73),(.06,.08,.045),eye,'head')
   part('fang',(side*.18,1.05,.41),(.065,.065,.16),mats[3],'head',True,direction=(0,0,-1))
   part('ear',(side*.19,.57,.96),(.12,.11,.19),mats[1],'head',True)
 elif index==1:
  # Flat armored crab: no canine muzzle, canine ears, or wolf fangs.
  part('crab_face',(0,.72,.56),(.4,.18,.16),mats[1],'head')
  for side in [-1,1]:
   link('eyestalk',(side*.21,.75,.66),(side*.29,.84,.9),.046,mats[0],'head')
   part('eye',(side*.29,.85,.9),(.065,.055,.055),eye,'head')
   link('claw_arm',(side*.44,.33,.53),(side*.73,.79,.49),.12,mats[0],'head')
   part('pincer_palm',(side*.76,.91,.5),(.24,.29,.17),mats[1],'head')
   part('pincer_outer',(side*.93,1.17,.51),(.085,.28,.11),mats[3],'head',True,direction=(-side*.15,1,0))
   part('pincer_inner',(side*.65,1.17,.51),(.07,.24,.095),mats[2],'head',True,direction=(side*.2,1,0))
  part('shell_front',(0,.38,.84),(.59,.34,.19),mats[1])
 else:
  # A broad mushroom hood shelters an eyeless tentacle maw, not a dog's face.
  part('round_fungal_head',(0,.73,.62),(.32,.22,.25),mats[0],'head')
  part('hood_stalk',(0,.66,.87),(.13,.14,.23),mats[3],'head')
  part('hood_cap',(0,.7,1.02),(.46,.39,.14),mats[1],'head')
  for n in range(7):
   a=n*math.tau/7;part('hood_spot',(.31*math.cos(a),.7+.26*math.sin(a),1.11),(.048,.045,.027),mats[2],'head')
  part('maw',(0,.94,.55),(.23,.04,.16),mats[1],'head')
  for n in range(6):
   x=(n-2.5)*.085
   link('jaw_tendril',(x,.93,.61),(x*1.35,1.09,.38),.038,mats[2],'head')
   link('jaw_curl',(x*1.35,1.09,.38),(x*1.05,1.17,.44),.028,mats[3],'head')
 legcount=6 if index==2 else 4
 for n in range(legcount):
  side=-1 if n%2==0 else 1;y=.4-(n//2)*(.42 if index==2 else .85)
  hip=(side*width*.8,y,.57);knee=(side*(width+.08),y-.06,.27);foot=(side*(width+.13),y+.09,.085)
  upper=f'leg{n}';lower=f'foot{n}';bones[upper]=(hip,knee,'body');bones[lower]=(knee,foot,upper)
  link('upper_leg',hip,knee,.11,mats[0],upper);link('lower_leg',knee,foot,.075,mats[1],lower)
  part('paw',foot,(.15,.21,.085),mats[0],lower)
  for claw in [-1,0,1]:part('claw',(foot[0]+claw*.062,foot[1]+.15,.06),(.028,.095,.035),mats[3],lower)
 link('tail',(0,-.48,.64),(0,-1.12,.6),.09,mats[1],'tail')
 if index==0:
  for n in range(9):
   y=-.6+n*.13;part('lava_mane',(0,y,.95),(.115,.09,.21+(.05 if n%2 else 0)),mats[2],cone=True,direction=(0,-.4,1))
  for side in [-1,1]:
   for n in range(4):part('magma_plate',(side*.28,-.4+n*.22,.72),(.075,.12,.08),mats[2])
 elif index==1:
  for n in range(5):part('shell_plate',(0,-.4+n*.19,.9),(.47,.16,.12),mats[1])
  for side in [-1,1]:
   for n in range(3):
    base=(side*.32,-.43+n*.28,.96)
    link('coral_stem',base,(base[0]+side*.08,base[1],1.2),.048,mats[2],'body')
    link('coral_branch',(base[0]+side*.04,base[1],1.09),(base[0]+side*.18,base[1]+.06,1.17),.037,mats[2],'body')
 else:
  for n in range(6):
   x=(-1 if n%2 else 1)*(.2 if n<4 else .07);y=-.5+(n//2)*.3;h=1.05+.07*(n%2)
   link('mushroom_stalk',(x,y,.79),(x,y,h),.065,mats[3],'body')
   part('mushroom_cap',(x,y,h),(.22,.2,.095),mats[1]);part('spore',(x+.05,y-.03,h+.085),(.035,.035,.023),mats[2])
  for side in [-1,1]:part('venom_sac',(side*.36,-.2,.65),(.12,.2,.2),mats[2])
 # Normalize entire authored creature to exactly unit height, feet resting at y-up origin after export.
 high=max((o.matrix_world@v.co).z for o in parts for v in o.data.vertices);factor=1/high
 for o in parts:
  o.location*=factor
  for v in o.data.vertices:v.co*=factor
 arm=bpy.data.armatures.new(kind+'_skeleton');rig=bpy.data.objects.new(kind+'_rig',arm);bpy.context.collection.objects.link(rig)
 bpy.context.view_layer.objects.active=rig;rig.select_set(True);bpy.ops.object.mode_set(mode='EDIT')
 for name,(head,tail,parent) in bones.items():
  b=arm.edit_bones.new(name);b.head=Vector(head)*factor;b.tail=Vector(tail)*factor
  if parent:b.parent=arm.edit_bones[parent]
 bpy.ops.object.mode_set(mode='OBJECT');bpy.ops.object.select_all(action='DESELECT')
 for o in parts:o.select_set(True)
 bpy.context.view_layer.objects.active=parts[0];bpy.ops.object.join();mesh=bpy.context.object;mesh.name=kind+'_skinned_mesh'
 mesh.parent=rig;mod=mesh.modifiers.new('Creature skin','ARMATURE');mod.object=rig
 for clip in ['Idle','Walk','Attack','Hit']:
  action=bpy.data.actions.new(clip);rig.animation_data_create();rig.animation_data.action=action
  for frame in range(0,25,2):
   t=frame/24;p=math.sin(t*math.tau)
   for pb in rig.pose.bones:pb.rotation_mode='XYZ';pb.rotation_euler=(0,0,0);pb.location=(0,0,0)
   rig.pose.bones['body'].location.z=(.012 if clip=='Idle' else .025)*p*factor
   for n in range(legcount):
    stride=math.sin(t*math.tau+(n%2)*math.pi+(n//2)*math.pi)
    rig.pose.bones[f'leg{n}'].rotation_euler.x=stride*({'Idle':.035,'Walk':.48,'Attack':.16,'Hit':.1}[clip])
    rig.pose.bones[f'foot{n}'].rotation_euler.x=-max(0,stride)*.35
   rig.pose.bones['tail'].rotation_euler.z=.17*p
   if clip=='Attack':rig.pose.bones['head'].rotation_euler.x=-.5*math.sin(t*math.pi);rig.pose.bones['body'].rotation_euler.x=.16*math.sin(t*math.pi)
   elif clip=='Hit':rig.pose.bones['head'].rotation_euler.z=.25*math.sin(t*math.pi);rig.pose.bones['body'].rotation_euler.y=.18*math.sin(t*math.pi)
   else:rig.pose.bones['head'].rotation_euler.z=.04*p
   for pb in rig.pose.bones:pb.keyframe_insert('rotation_euler',frame=frame);pb.keyframe_insert('location',frame=frame)
  track=rig.animation_data.nla_tracks.new();track.name=clip;track.strips.new(clip,0,action);track.mute=True
 rig.animation_data.action=None
 for pb in rig.pose.bones:pb.rotation_euler=(0,0,0);pb.location=(0,0,0)
 bpy.ops.object.select_all(action='DESELECT');rig.select_set(True);mesh.select_set(True);bpy.context.view_layer.objects.active=rig
 path=OUT/f'adventure_{kind}.glb'
 bpy.ops.export_scene.gltf(filepath=str(path),export_format='GLB',use_selection=True,export_animations=True,export_animation_mode='NLA_TRACKS',export_frame_range=False,export_force_sampling=True)
 # Explicit clip-index contract, independent of Blender exporter track ordering.
 raw=path.read_bytes();jlen,jtype=struct.unpack_from('<II',raw,12);doc=json.loads(raw[20:20+jlen]);animations=doc['animations'];doc['animations']=sorted(animations,key=lambda a:['Idle','Walk','Attack','Hit'].index(a['name']))
 payload=json.dumps(doc,separators=(',',':')).encode();payload+=b' ' *((-len(payload))%4);tail=raw[20+jlen:];path.write_bytes(struct.pack('<III',0x46546c67,2,20+len(payload)+len(tail))+struct.pack('<II',len(payload),jtype)+payload+tail)
 manifest.append({'file':path.name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'origin':'Original procedural Blender geometry; no imported geometry or external artwork','license':'CC0-1.0','author':'Project contributors','height_m':1,'forward':'-Z','up':'+Y','meshes':len(doc.get('meshes',[])),'skins':len(doc.get('skins',[])),'bones':len(bones),'vertices':len(mesh.data.vertices),'triangles':sum(len(p.vertices)-2 for p in mesh.data.polygons),'animations':[{'index':i,'name':a['name'],'authored_poses':13,'sampled_frames':max(doc['accessors'][s['input']]['count'] for s in a['samplers'])}for i,a in enumerate(doc['animations'])]})
 rig.location.x=(index-1)*2.1;all_roots.append(rig)
 for track in rig.animation_data.nla_tracks:track.mute=True
 return rig
for i,kind in enumerate(COLORS):build(kind,i)
OUT.joinpath('adventure-manifest.json').write_text(json.dumps({'generator':'tools/build_adventure_models.py','assets':manifest},indent=2)+'\n')
# Actual model review contact sheet, no source screenshots or synthetic pasted sprites.
bpy.ops.mesh.primitive_plane_add(size=200);floor=bpy.context.object;floor.location.z=-.01;floor.data.materials.append(material('studio_floor',(.035,.045,.05)))
for loc,power,size,color in [((0,-3,6),900,5,(1,.87,.72)),((0,4,4),700,4,(.69,.82,1))]:
 bpy.ops.object.light_add(type='AREA',location=loc);o=bpy.context.object;o.data.energy=power;o.data.shape='DISK';o.data.size=size;o.data.color=color;o.rotation_euler=(Vector((0,0,.4))-o.location).to_track_quat('-Z','Y').to_euler()
bpy.ops.object.camera_add(location=(3.5,6.8,3.5));camera=bpy.context.object;camera.rotation_euler=(Vector((0,0,.5))-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.type='ORTHO';camera.data.ortho_scale=7.1;bpy.context.scene.camera=camera
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=24;scene.render.resolution_x=1600;scene.render.resolution_y=700;scene.render.resolution_percentage=100;scene.world.color=(.12,.12,.12);scene.render.filepath=str(ROOT/'screenshots/adventure-original-models-contact.png');bpy.ops.render.render(write_still=True)
print('ADVENTURE_MODELS_READY '+json.dumps(manifest))
