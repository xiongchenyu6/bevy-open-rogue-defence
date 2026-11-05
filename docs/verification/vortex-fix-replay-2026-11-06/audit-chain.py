#!/usr/bin/env python3
"""Read archived actual actor/contact data; f32 distance audit, no game execution."""
import gzip, json, pathlib, re, struct
root=pathlib.Path(__file__).resolve().parent
f=lambda x:struct.unpack('f',struct.pack('I',x))[0]
f32=lambda x:struct.unpack('f',struct.pack('f',x))[0]
def distance2(a,b):
 d=[f32(x-y) for x,y in zip(a,b)]
 return f32(f32(f32(d[0]*d[0])+f32(d[1]*d[1]))+f32(d[2]*d[2]))
def xyz(row):return [f(x) for x in row[3:6]]
report={'scope':'Read-only distance audit; telemetry lag from source scheduling; elite flag is unrecorded and eligibility remains conditional','cases':{}}
for case in 'ab':
 folder=root/f'replay-{case}';rows={}
 with gzip.open(folder/'active-h2-behind-sa11ce-c5-t0.jsonl.gz','rt') as stream:
  for line in stream:
   if not line.startswith('{'):continue
   r=json.loads(line)
   if r.get('type')=='trace' and r['frame'] in (12710,12711,12715):rows[r['frame']]=r
   if r.get('type')=='trace' and r['frame']>12715:break
 primary=None
 with gzip.open(folder/'active-h2-behind-sa11ce-c5-t0.stderr.gz','rt') as stream:
  for line in stream:
   if 'PROJECTILE_HIT' not in line:continue
   clean=re.sub(r'\x1b\[[0-9;]*m','',line)
   if f'elapsed_bits={rows[12711]["elapsed_bits"]}' in clean and 'skill_index: Some(0)' in clean:
    primary=int(re.search(r'enemy_bits=(\d+)',clean).group(1));break
 assert primary is not None
 cast={r[0]:r for r in rows[12710]['enemies']};point=xyz(cast[primary])
 candidates=sorted((distance2(point,xyz(row)),row[0]) for row in cast.values() if row[0]!=primary)
 assert candidates[0][0]<candidates[1][0], 'unique nearest candidate required'
 intended=candidates[0][1];previous={r[0]:r for r in rows[12715]['enemies']}
 strike=xyz(previous[intended]);strike[1]=f32(strike[1]+0.18)
 neighbor_distance=distance2(strike,xyz(previous[primary]))
 actual=json.loads((root/f'first-difference-contacts-{case}.json').read_text())[0]
 report['cases'][case]={
  'cast_frame_inferred_from_precast_telemetry_copy':12710,'telemetry_update_frame':12711,
  'primary_actual_contact_id':primary,'primary_xyz_at_cast':point,
  'unvisited_distance_candidates':candidates,'unique_nearest_intended_id':intended,
  'strike_origin_using_previous_frame_target_plus_y018':strike,
  'neighbor_distance_squared':neighbor_distance,'normal_radius_squared':f32(.53*.53),'elite_radius_squared':1.0,
  'normal_target_id':intended,'neighbor_id':primary,'lower_entity_bits_if_both_overlap':min(primary,intended),
  'actual_contact_target_id':actual['enemy_bits'],'actual_progress_bits':actual['progress_bits'],
  'neighbor_elite_flag_recorded':False,
 }
(root/'chain-distance-audit.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
