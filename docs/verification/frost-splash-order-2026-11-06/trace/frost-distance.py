from pathlib import Path
import json,struct
r=Path(__file__).parent;d=json.loads((r/'frost-link-raw.json').read_text()); f=lambda n:struct.unpack('<f',struct.pack('<I',n))[0];q=lambda x:struct.unpack('<f',struct.pack('<f',x))[0];bits=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
out={}
for s in 'ab':
 v=d[s];row=v['frames']['94474'];es={x[0]:x for x in row['enemies']};parent=next(e for e in v['contacts'] if e['index']==93225);origin=es[int(parent['enemy_bits'])]; nearby=[]
 for a in es.values():
  dx=q(f(a[3])-f(origin[3]));dz=q(f(a[5])-f(origin[5]));dist=q(q(dx*dx)+q(dz*dz))
  if a[0]!=origin[0] and dist<=9:
   nearby.append({'id':a[0],'xyz_bits':a[3:6],'xyz':list(map(f,a[3:6])),'d2':dist,'d2_bits':bits(dist)})
 nearby.sort(key=lambda a:(a['d2'],a['id']))
 child=next(x for x in row['projectiles'] if x[0]==17179855841)
 out[s]={'parent_contact':parent,'origin':origin,'origin_xyz':list(map(f,origin[3:6])),'all_neighbor_candidates_sorted_distance_ID':nearby,'top6':nearby[:6],'child_at_end_parent_frame':child,'child_xyz':list(map(f,child[1:4]))}
 print(s,'parent',parent['index'],origin[0],out[s]['origin_xyz']);print('top6',[(x['id'],x['xyz'],x['d2_bits']) for x in nearby[:6]])
(r/'frost-distance-evidence.json').write_text(json.dumps(out,indent=2))
