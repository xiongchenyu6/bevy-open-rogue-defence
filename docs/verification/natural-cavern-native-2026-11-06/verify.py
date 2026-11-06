"""Offline evidence checks; no game, renderer, source mutation or art acceptance."""
import hashlib,json,pathlib,struct,subprocess
p=pathlib.Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda f:json.loads((p/f).read_text())
for n,h in read('files.sha256.json').items():assert sha((p/n).read_bytes())==h,n
meta=read('provenance.json');base=meta['base_source_commit']
assert not any(meta[k] for k in ['candidate_visual_accepted','camera_adopted','formal_assets_changed'])
source=read('source-inputs.json');assert source==read('build/build-inputs-before.json')==read('build/build-inputs-after.json')
repo=p.parents[2]
for n,h in source.items():
 data=(p/'capture-demon-cave.rs').read_bytes() if n==meta['source_override'] else subprocess.check_output(['git','show',base+':'+n],cwd=repo)
 assert sha(data)==h,n
commands=read('build/commands.json');assert len(commands)==2 and all(c['exit']==0 for c in commands)
binary=repo/meta['binary_original']
if binary.exists():assert sha(binary.read_bytes())==read('build/binary.json')['sha256']
assert 'Finished `release`' in (p/'build/clippy.log').read_text() and 'Finished `dev`' in (p/'build/build.log').read_text()
case_results=read('results.json');assert len(case_results)==2 and all(c['exit']==0 for c in case_results)
for name in ['production-camera','wide-camera-proposal']:
 text=(p/name/'game.log').read_text()
 assert '[capture] PASS den-preview:'in text and 'wrote 400 simulation frames'in text
 assert 'panicked'not in text and 'ERROR'not in text
 assert len(list((p/name/'frames').glob('*.png')))==9
assert 'eye=Vec3(15.0, 24.0, 21.0) center_x=7 vertical=26'in (p/'wide-camera-proposal/game.log').read_text()
baseline=read('original-den-baseline/result.json');assert baseline['exit']==0
text=(p/'original-den-baseline/game.log').read_text();assert 'PASS demon cave quiet:'in text and 'PASS demon-cave:'in text and 'wrote 320 simulation frames'in text
assert 'panicked'not in text and 'ERROR'not in text and len(list((p/'original-den-baseline/frames').glob('*.png')))==7
candidate=(p/'REJECTED-candidate.glb').read_bytes();c=read('candidate.json');assert sha(candidate)==c['sha256']=='d058912a9f56a9db4dfaf86d8ce8c03d08de855ed73faa556058a2ed38c900cf'
assert len(candidate)==c['bytes']==2470092
_,version,length=struct.unpack_from('<III',candidate);assert version==2 and length==len(candidate)
size,kind=struct.unpack_from('<II',candidate,12);assert kind==0x4e4f534a
doc=json.loads(candidate[20:20+size]);assert not doc.get('extensionsRequired') and all('uri'not in x for x in doc['images'])
assert all('TEXCOORD_0'in x['attributes'] for m in doc['meshes'] for x in m['primitives'])
for m in doc['materials']:
 assert m.get('alphaMode','OPAQUE')=='OPAQUE' and abs(m['pbrMetallicRoughness'].get('roughnessFactor',1)-.97)<1e-6
report,_=json.JSONDecoder().raw_decode((p/'independent-candidate-check.log').read_text())
assert report['sha256']==c['sha256'] and report['existing_models_unchanged_count']==52 and report['production_path_audit']['collisions']==[]
assets=read('asset-manifest.json');assert len(assets)==189
for n,h in assets.items():
 data=candidate if n=='models/environment_demon_cavern_preview.glb' else subprocess.check_output(['git','show',base+':assets/'+n],cwd=repo)
 assert sha(data)==h,n
print('PASS native 2x400 plus original320, saved PNGs, exact inputs, candidate payload/SAT; artwork and camera remain REJECTED')
