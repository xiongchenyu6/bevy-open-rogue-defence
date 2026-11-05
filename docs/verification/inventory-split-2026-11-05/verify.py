#!/usr/bin/env python3
"""Read-only validation of archived native UI evidence; launches no game."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parent
cases={
'equipment-en-desktop':310,'equipment-zh-compact':310,'equipment-en-portrait':310,
'crafting-0-en-desktop':310,'crafting-1-zh-desktop':310,'crafting-1-en-compact':310,
'crafting-0-zh-compact':310,'crafting-1-zh-portrait':310,
'controller-desktop':2400,'controller-compact':2400,'controller-scroll-compact':1600}
for name,count in cases.items():
 text=(root/'after'/name/'capture.log').read_text()
 marker=('CONTROLLER_ROUTE_PASS all 13 steps' if name=='controller-scroll-compact' else
         'CONTROLLER_ROUTE_PASS all 80 steps' if name.startswith('controller') else
         'PASS equipment:' if name.startswith('equipment') else 'PASS '+name[:10]+':')
 assert marker in text,(name,marker)
 assert f'wrote {count} simulation frames' in text,name
 assert 'panicked at' not in text,name
 sequence=(root/'native-sequence.log').read_text()
 assert 'PASS '+name in sequence,name
scroll=(root/'after/controller-scroll-compact/capture.log').read_text()
assert 'CONTROLLER_ROUTE_CHECK scroll-down PASS' in scroll
assert 'CONTROLLER_ROUTE_CHECK scroll-up PASS' in scroll
assert 'scroll-down before=0 after=200.00002' in scroll
assert 'scroll-up before=200.00002 after=0.000011444092' in scroll
before=(root/'before-controller-compact/capture.log').read_text()
assert 'controller route stalled at step 70:' in before and 'Crafting recipe 3' in before
assert '673 passed; 0 failed; 1 ignored' in (root/'full-tests.log').read_text()
clippy=(root/'clippy.log').read_text()
assert 'Finished `release` profile' in clippy and 'error:' not in clippy
assert 'PASS all native UI/controller cases' in (root/'native-manager.log').read_text()
manifest=json.loads((root/'artifact-hashes.json').read_text())
for name,expected in manifest.items():
 assert hashlib.sha256((root/name).read_bytes()).hexdigest()==expected,name
print(f'PASS archived UI evidence: {len(cases)} native cases, {sum(cases.values())} completed updates; controller 80/80/13 steps; scroll down/up; 673 library tests; strict Clippy. Synthetic controller only.')
