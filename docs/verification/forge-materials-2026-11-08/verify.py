import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
plan=json.loads((D/'collection-plan.json').read_text());raw={}
for x in plan['files']:
 b=gzip.decompress((D/x['archive']).read_bytes());assert len(b)==x['bytes'] and sha(b)==x['sha256'];raw[x['source']]=b
for n in plan['selected_images']:assert (D/'frames'/n).exists()
def obj(n):return json.loads(raw[n])
for stage in ['capture-clippy','capture-build','native','library-tests','release-clippy']:assert obj('r5/'+stage+'-result.json')['returncode']==0,stage
inputs=obj('r5/inputs.json');assert inputs==obj('r5/inputs-after.json')
for key,b in raw.items():
 if key.startswith('r5/source/'):assert sha(b)==inputs[key.removeprefix('r5/source/')],key
recipes={0:['Bloodreaver','StormBastion','NightCrown'],1:['AstralPrism','Suncoil','WorldEngine']}
for chain,size,lang in [(0,'1280x720','en'),(1,'1280x720','zh'),(0,'360x800','en'),(1,'360x800','zh'),(0,'320x667','en'),(1,'320x667','zh'),(1,'852x393','zh')]:
 name=f'after-chain{chain}-{size}-{lang}';assert obj('r5/'+name+'-result.json')['returncode']==0
 text=raw['r5/'+name+'.log'].decode();rows=re.findall(r'actual recipe (\w+): inventory=(\d+); authoritative effects=(.*)',text)
 assert [x[0] for x in rows]==recipes[chain] and [int(x[1]) for x in rows]==[4,3,1]
 baseline='before-chain0-1280x720-en.log' if chain==0 else 'before-chain1-360x800-zh.log'
 old=re.findall(r'actual recipe (\w+): inventory=(\d+); authoritative effects=(.*)',raw[baseline].decode());assert rows==old,'UI change must retain all three actual recipe stat snapshots'
 assert 'five real ingredient purchases' in text and 'ingredients consumed' in text and 'duplicate prevented' in text and 'close resumes' in text
 assert re.search(r'MATERIAL_ART count=[23] atlas_loaded=true bounded=true',text)
assert obj('build-result.json')['returncode']==101 and obj('r2/build-result.json')['returncode']==101
isolated=obj('r5/isolated-library-tests-result.json');assert isolated['returncode']==0
assert len(isolated['preference_overrides'])==5
assert re.search(rb'786 passed; 0 failed; 1 ignored',raw['r5/isolated-library-tests.log'])
assert len(inputs)==402
assert 'r5/source.patch' in raw
assert len([k for k in raw if k.startswith('r5/source/') and k.endswith('.rs')])==6
print('PASS exact archive/raw bytes, frozen source snapshots, seven real UI chains and three authoritative recipe snapshots per chain.')
