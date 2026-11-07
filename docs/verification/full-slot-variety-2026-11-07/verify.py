"""Portable integrity and independently recomputed observed results."""
import ast,gzip,hashlib,importlib.util,json,re,sys,struct
from pathlib import Path
sys.dont_write_bytecode=True
D=Path(sys.argv[1]).resolve()
def sha(data):return hashlib.sha256(data).hexdigest()
def load(name):return json.loads((D/name).read_text())
def log(name):return gzip.decompress((D/'raw'/(name+'.log.gz')).read_bytes()).decode()
def helper(name):
 spec=importlib.util.spec_from_file_location(name,D/(name+'.py'));module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
files=load('SHA256.json')
assert set(files)=={str(p.relative_to(D)) for p in D.rglob('*') if p.is_file() and p.name!='SHA256.json'}
for name,digest in files.items():assert sha((D/name).read_bytes())==digest,name
for name,row in load('raw-log-manifest.json').items():
 data=gzip.decompress((D/name).read_bytes());assert len(data)==row['bytes'] and sha(data)==row['sha256'],name
raw=D/'raw';matrix=helper('inspect_matrix');repeat=helper('compare_repeat');sampler=helper('verify_sampler');source=helper('verify_source')
root=raw/'matched-inputs/target/verification'
for stage,batch,count in [('baseline','full-slot-baseline-c1-2026-11-07',12),('after','full-slot-after-c1-2026-11-07',12),('repeat','full-slot-after-repeat-c1-2026-11-07',1)]:
 result=matrix.inspect(root/batch/'summary.json',count)
 assert json.loads(json.dumps(result))==load('analysis/'+('repeat-case-analysis.json' if stage=='repeat' else stage+'-analysis.json'))
 summary=json.loads((root/batch/'summary.json').read_text())
 pin=(raw/('baseline-build/binary.sha256' if stage=='baseline' else 'candidate-r9/sim-binary.sha256')).read_text().strip()
 assert summary['binary_sha256']==pin and all(c['binary_sha256']==pin for c in summary['cases'])
assert source.verify(raw)==load('analysis/source-analysis.json')
assert load('raw/candidate-r9/inputs.json')==load('raw/candidate-r9/inputs-after-repeat.json')
content=D/'source-unchanged/src/game/content.rs'
assert sha(content.read_bytes())==load('raw/candidate-r9/inputs.json')['src/game/content.rs']
assert json.loads(json.dumps(sampler.verify(raw/'candidate-r9/focused.log.gz',content)))==load('analysis/sampler-analysis-r9.json')
a=root/'full-slot-after-c1-2026-11-07/active-h1-behind-s1-c0-t0.jsonl.gz';b=root/'full-slot-after-repeat-c1-2026-11-07/active-h1-behind-s1-c0-t0.jsonl.gz'
comparison=repeat.compare(a,b);assert comparison==load('analysis/repeat-analysis.json')
assert comparison['status']=='RECORDED_PROJECTION_MISMATCH' and not comparison['recorded_projection_equal']
first=comparison['first_difference_original_records']
assert first['record_index']==80 and first['left']['frame']==38288 and first['left']['elapsed']==625.47284
assert load('raw/before/result.json')['returncode']==101 and '0 passed; 2 failed;' in log('before/tests')
for stage in ('candidate-r1','candidate-r4','candidate-r5','candidate-r6'):
 name='focused' if stage=='candidate-r1' else 'release-clippy'
 assert load('raw/'+stage+'/'+name+'-result.json')['returncode']==101
for stage,name in [('candidate-r2','1280x720-en-Sr'),('candidate-r3','360x800-en-Ssr'),('candidate-r7','320x667-en-Ssr')]:assert load('raw/'+stage+'/'+name+'-result.json')['returncode']==101
assert load('raw/candidate-r8/focused-observed-exit.json')['returncode']==0
assert load('raw/candidate-r8/inputs.json')==load('raw/candidate-r8/inputs-at-intentional-stop.json')
for stage,name in [('baseline-build','bin-tests'),('baseline-build','build'),('baseline-build','matrix')]+[('candidate-r9',n) for n in ('release-clippy','focused','build','library-tests','capture-clippy','bin-tests','sim-build','after-matrix','after-repeat')]:assert load('raw/'+stage+'/'+name+'-result.json')['returncode']==0,(stage,name)
assert '781 passed; 0 failed; 1 ignored;' in log('candidate-r9/library-tests')
assert '8 passed; 0 failed;' in log('candidate-r9/focused') and '6 passed; 0 failed;' in log('candidate-r9/bin-tests')
for name in ('baseline-build','candidate-r9'):
 state=load('raw/'+name+'/unit-state.json')['terminal_systemctl'];assert state['ActiveState']=='inactive' and state['Result']=='success' and state['ExecMainStatus']=='0'
before=re.search(r'FULL_SLOT_EARNED_TRAINING before=(\([^()\n]+\)) old=SkillId\(\d+\) rank=4',log('candidate-r9/focused'))
after=re.search(r'FULL_SLOT_EARNED_TRAINING after_relearn=(\([^()\n]+\))',log('candidate-r9/focused'))
assert before and after and ast.literal_eval(before[1])==ast.literal_eval(after[1])
N=r'-?\d+(?:\.\d+)?(?:e[+-]?\d+)?'
rect=re.compile(r'FULL_SLOT_RECT name="[^"]+" center=Vec2\(('+N+r'), ('+N+r')\) size=Vec2\(('+N+r'), ('+N+r')\) card_center=Vec2\(('+N+r'), ('+N+r')\) card_size=Vec2\(('+N+r'), ('+N+r')\)')
glyph=re.compile(r'FULL_SLOT_GLYPH_RECT .*? center=Vec2\(('+N+r'), ('+N+r')\) glyph=Vec2\(('+N+r'), ('+N+r')\) card_center=Vec2\(('+N+r'), ('+N+r')\) card_size=Vec2\(('+N+r'), ('+N+r')\)')
def contained(matches,count):
 assert len(matches)==count
 for m in matches:
  x,y,w,h,cx,cy,cw,ch=map(float,m.groups());assert min(w,h,cw,ch)>0;assert abs(x-cx)+w*.5<=cw*.5+1 and abs(y-cy)+h*.5<=ch*.5+1
swaps=['1280x720-en-Sr','1280x720-zh-Ssr','360x800-en-Ssr','360x800-zh-Sr','320x667-en-Ssr']
for name in swaps+['ordinary-320x667','ordinary-360x800']:
 assert load('raw/candidate-r9/'+name+'-result.json')['returncode']==0
 text=log('candidate-r9/'+name);assert 'wrote 180 simulation frames' in text and 'gold=0 paid_draws=1' in text
 contained(list(rect.finditer(text)),9);contained(list(glyph.finditer(text)),3)
 if name in swaps:assert re.search(r'PASS full-slot-variety actual-G paid80 pointer-swap old=\d+ new=\d+ rarity=(Sr|Ssr) old_rank=0 new_rank=1 attack_preserved=true',text)
 else:assert 'PASS full-slot-ordinary actual-G paid80 pointer-upgrade SKY JUDGMENT4→5 no-swap-hand' in text
for lang in ('en','zh'):
 name='long-320-'+lang;assert load('raw/candidate-r9/'+name+'-result.json')['returncode']==0
 assert 'wrote 240 simulation frames' in log('candidate-r9/'+name) and 'PASS phone-draft:' in log('candidate-r9/'+name)
for size in ('1280x720','360x800'):
 for lang in ('en','zh'):
  name='native-before-r2/'+size+'-'+lang;assert load('raw/'+name+'-result.json')['returncode']==0
  assert 'wrote 180 simulation frames' in log(name)
for entry in load('formatter-reconstruction.json')['files']:
 data=(D/'source-reconstructed'/entry['stage']/entry['file']).read_bytes()
 assert entry['matched'] and sha(data)==entry['historical_sha256']==load('raw/'+entry['stage']+'/inputs.json')[entry['file']]
for name in ('src/bin/capture/market_wallet.rs','src/bin/capture/skill_identity.rs'):assert sha((D/'source-unchanged'/name).read_bytes())==load('raw/baseline-build/inputs.json')[name]==load('raw/candidate-r9/inputs.json')[name]
views=load('root-viewed-images.json');assert len(views)==17
assert {str(p.relative_to(D/'frames')) for p in (D/'frames').rglob('*.png')}=={r['path'] for r in views}
for row in views:
 data=(D/'frames'/row['path']).read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n'
 size=struct.unpack('>II',data[16:24]);expected=(360,800) if '360x800' in row['path'] else (320,667) if '320' in row['path'] else (1280,720)
 assert size==expected,(row['path'],size)
print('PASS archive integrity,25 uncensored simulation cases,4608 sampler rows,9 native cases; recorded-repeat mismatch faithfully retained')
