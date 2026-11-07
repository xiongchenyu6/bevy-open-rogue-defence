"""Check unchanged scalar rules and recorded final source byte identities."""
import argparse
import hashlib
import json
from pathlib import Path
import re

def sha(data):return hashlib.sha256(data).hexdigest()
def function(source,name):
    match=re.search(r'\bfn\s+'+re.escape(name)+r'\s*\(',source);assert match,name
    start=source.index('{',match.end());depth=0
    for i in range(start,len(source)):
        depth += (source[i]=='{')-(source[i]=='}')
        if depth==0:return source[match.start():i+1]
    raise AssertionError(name)
def verify(root):
    before=json.loads((root/'baseline-build/inputs.json').read_text());after=json.loads((root/'candidate-r9/inputs.json').read_text())
    assert before==json.loads((root/'baseline-build/inputs-after.json').read_text())
    assert after==json.loads((root/'candidate-r9/inputs-after.json').read_text())
    assert before['src/bin/balance_sim.rs']==after['src/bin/balance_sim.rs']
    assert {k:v for k,v in before.items() if k.startswith('assets/')}=={k:v for k,v in after.items() if k.startswith('assets/')}
    old=(root/'baseline-build/source/src/game/progression.rs').read_text();new=(root/'candidate-r9/source/src/game/progression.rs').read_text()
    unchanged=['skill_cost','bond_cost','skill_rarity_available','choose_weighted_skill_for_roll','grant_skill_rank']
    for name in unchanged:assert function(old,name)==function(new,name),name
    for stage,inputs in [('baseline-build',before),('candidate-r9',after)]:
        for path in (root/stage/'source').rglob('*.rs'):
            name=str(path.relative_to(root/stage/'source'));assert sha(path.read_bytes())==inputs[name],(stage,name)
    return {'unchanged_functions':unchanged,'assets_unchanged':True,'logger_identical':True,'changed_inputs':[name for name in sorted(set(before)|set(after)) if before.get(name)!=after.get(name)],'scope':'Stored source byte equality and unchanged scalar helper bodies. Actual eligibility is separately checked against raw4608 sampler hands; no common-helper arithmetic is mirrored.'}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=Path);a=p.parse_args();print(json.dumps(verify(a.root),indent=2))
