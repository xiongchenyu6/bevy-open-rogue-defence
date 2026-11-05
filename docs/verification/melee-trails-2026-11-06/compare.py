import pathlib,re,json
p=pathlib.Path(__file__).resolve().parent
report={}
for kind in ['axe','void','rift','tide','sword','ember']:
 a=(p/f'before-{kind}.log').read_text();b=(p/f'after-{kind}.log').read_text()
 rows=lambda text:[line for line in text.splitlines() if line.startswith('ATTACK_TIMING ')]
 differences=[dict(before=x,after=y) for x,y in zip(rows(a),rows(b)) if x!=y]
 if kind!='void':
  assert rows(a)==rows(b),(kind,'actualHP/animation/contact/cadence changed')
 else:
  assert len(rows(a))==len(rows(b))
  for x,y in zip(rows(a),rows(b)):
   mx=re.match(r'ATTACK_TIMING frame=(\d+) health=([\d.]+) damage=([\d.]+) clips=(.*)',x)
   my=re.match(r'ATTACK_TIMING frame=(\d+) health=([\d.]+) damage=([\d.]+) clips=(.*)',y)
   assert mx[1]==my[1] and mx[4]==my[4]
   assert abs(float(mx[2])-float(my[2]))<=.032 and abs(float(mx[3])-float(my[3]))<=.032
  contact=json.loads((p/'void-contact-comparison.json').read_text())
  assert contact['actual_contact_fields_equal'] and contact['contacts']==77
 trail=lambda text:[dict(frame=int(m[1]),entity=m[2],alpha=float(m[3]),mode=m[4],materials=int(m[5])) for m in re.finditer(r'SWORD_TRAIL frame=(\d+) entity=(\S+) alpha=([\d.]+) mode=(\w+) material_count=(\d+)',text)]
 before,after=trail(a),trail(b)
 if kind!='ember':
  assert after and all(r['mode']=='Blend' and r['alpha']<1 for r in after)
  assert len({r['materials'] for r in after})==1,'Actual render material assets grow across attacks'
  assert min(r['alpha'] for r in after)<.1,'Actual afterimage never fades'
 report[kind]=dict(actual_hp_and_clip_rows_identical=(not differences),aggregate_rounding_differences=differences,recorded_frames=len(rows(a)),before_trail_rows=before,after_trail_rows=after)
(p/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS five exact HP/clip controls; Void77actualcontacts equal with documented aggregate f32sum difference; melee Blend fade/materialcounts constant')
