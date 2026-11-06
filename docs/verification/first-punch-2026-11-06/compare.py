from pathlib import Path
import re,json
p=Path(__file__).resolve().parent
texts=[(p/(x+'-native.log')).read_text()for x in ['before','after']]
def contacts(t):
 rows=[]
 for s in t.splitlines():
  m=re.search(r'HIT_MATERIAL_CONTACT frame=(\d+) actor=\S+ hp=([\d.]+) damage=([\d.]+)',s)
  if m:rows.append((int(m[1]),float(m[2]),float(m[3])))
 return sorted(rows)
def timing(t):return [s for s in t.splitlines()if s.startswith('ATTACK_TIMING ')]
a,b=map(contacts,texts);ta,tb=map(timing,texts)
r={'contact_counts':[len(a),len(b)],'actual_contact_hp_damage_frame_multisets_equal':a==b,'timing_rows':[len(ta),len(tb)],'all_logged_80_frame_health_clip_phase_speed_equal':ta==tb,'contact_frame_count':[len({x[0]for x in a}),len({x[0]for x in b})],'rng_telemetry':False,'limitation':'No RNG state logged; no empirical RNG-equality claim. Entity allocation ignored for semantic contact comparison.'}
(p/'comparison.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2));assert a and a==b and ta and ta==tb
