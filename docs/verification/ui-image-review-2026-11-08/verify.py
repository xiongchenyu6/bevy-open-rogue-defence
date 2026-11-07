import gzip,hashlib,json,re,shutil,subprocess,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for name,h in m.items():assert sha((D/name).read_bytes())==h,name
plan=json.loads((D/'collection-plan.json').read_text());raw={}
for item in plan['files']:
 b=(D/item['archive']).read_bytes();b=gzip.decompress(b) if item['compressed'] else b
 assert sha(b)==item['sha256'] and len(b)==item['bytes'];raw[item['source']]=b
images=json.loads((D/'image-decodes.json').read_text())['images'];assert len(images)==8
for i in range(0,8,2):
 a,b=images[i:i+2];assert a['png_sha256']==b['png_sha256'] and a['decoded_rgba_sha256']==b['decoded_rgba_sha256']
for image in images:
 png=D/'originals'/Path(image['source']).relative_to('docs/verification/decision-phase-2026-11-08/frames/phase')
 assert sha(png.read_bytes())==image['png_sha256'];assert sha((D/image['jpeg']).read_bytes())==image['jpeg_sha256']
 if shutil.which('magick'):
  rgba=subprocess.check_output(['magick',str(png),'-depth','8','RGBA:-']);assert sha(rgba)==image['decoded_rgba_sha256']
base='target/verification/ui-layer-2026-11-08/'
def result(n):return json.loads(raw[base+n])['returncode']
for n in ['1280x720-result.json','360x800-result.json','serial/1280x720-result.json','serial/360x800-result.json','diagnostic-r2/1280x720-result.json','diagnostic-r2/360x800-result.json']:assert result(n)==0,n
assert result('diagnostic/result.json')==101
assert result('single-camera/result.json')==0 and result('single-camera/360x800-result.json')==101
assert base+'single-camera/1280x720-result.json' not in raw
for size in ['1280x720','360x800']:
 text=raw[base+'diagnostic-r2/'+size+'.log'].decode();assert 'missing_batches=0' in text and 'mismatched_indices=0' in text
 assert re.search(r'nodes=[1-9][0-9]*',text) and re.search(r'glyphs=[1-9][0-9]*',text)
print('PASS exact archive/raw bytes, four identical PNG pairs, eight JPEGs, preserved experiment outcomes and diagnostic traces.')
print('RGBA recomputation '+('PASS' if shutil.which('magick') else 'SKIPPED ImageMagick unavailable'))
