#!/usr/bin/env python3
"""Read actual first-difference rows; no simulation or target assignment injection."""
import json,math,pathlib,struct
root=pathlib.Path(__file__).resolve().parent
context=json.loads((root/'first-difference-context.json').read_text())
f=lambda bits:struct.unpack('f',struct.pack('I',bits))[0]
result={'scope':'Map actual projectile direction to recorded candidate positions; IDs and exact-X bits retained; semantic classes outside recorded fields remain unclaimed','cases':{}}
for case in 'ab':
 before=context['previous'][case];after=context['first'][case]
 old={r[0] for r in before['projectiles']};new=[r for r in after['projectiles'] if r[0] not in old];assert len(new)==1
 shot=new[0];origin=[f(x) for x in after['hero_positions'][0][1:]];velocity=[f(x) for x in shot[4:7]];speed=math.sqrt(sum(x*x for x in velocity));direction=[x/speed for x in velocity]
 candidates=[]
 for row in after['enemies']:
  pos=[f(x) for x in row[3:6]];delta=[x-y for x,y in zip(pos,origin)];distance=math.sqrt(sum(x*x for x in delta));error=sum((x/distance-y)**2 for x,y in zip(delta,direction))
  candidates.append({'entity_bits':row[0],'xyz':pos,'x_bits':row[3],'hp':f(row[1]),'direction_error_squared':error,'distance_squared':distance*distance})
 selected=min(candidates,key=lambda r:r['direction_error_squared']);assert selected['direction_error_squared']<1e-12
 front=min(r['xyz'][0] for r in candidates);tied=[r for r in candidates if r['xyz'][0]==front];assert len(tied)==3 and len({r['x_bits'] for r in tied})==1
 assert selected['entity_bits']==min(r['entity_bits'] for r in tied)
 result['cases'][case]={'frame':after['frame'],'hero_xyz':origin,'new_projectile':shot,'normalized_direction':direction,'selected_position_match':selected,'exact_front_tie':tied,'closest_front':min(tied,key=lambda r:r['distance_squared'])}
(root/'native-aim-audit.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
