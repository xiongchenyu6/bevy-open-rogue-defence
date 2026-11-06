#!/usr/bin/env python3
"""Portable, read-only recomputation from all 24 archived raw process logs."""
import csv
import gzip
import hashlib
import itertools
import json
import math
from pathlib import Path

D=Path(__file__).resolve().parent
load=lambda name:json.loads((D/name).read_text())
sha=lambda data:hashlib.sha256(data).hexdigest()
for name,value in load('SHA256.json').items():assert sha((D/name).read_bytes())==value,name
assert load('manager/process.json')['exit']==0
unit=(D/'manager/unit-terminal-state.txt').read_text()
assert 'Result=success' in unit and 'ExecMainStatus=0' in unit
scope=load('manager/scope.json');binary=load('manager/binary.json')
assert binary['source_commit']=='acbdd2d75c814c6e05ac398f5b3e025dea1b4662'
assert binary['sha256']=='cc31f1b4ecf07ebb96dee3c28c9479dec2e9641f72611a9b1b1b876f474fde84'
summary=load('summary.json');assert summary['binary_sha256']==binary['sha256']
assert summary['requested_cases']==24 and len(summary['cases'])==24
assert (summary['chapters'],summary['training'],summary['frames'],summary['jobs'])==([0],[0],120000,2)
assert summary['pressure_override']==[] and summary['require_telemetry'] is True
maxima=scope['max_ranks'];assert maxima==[5]*32
raw={}
for item in load('raw-manifest.json'):
    compressed=(D/item['archive']).read_bytes();assert sha(compressed)==item['gzip_sha256']
    data=gzip.decompress(compressed);assert len(data)==item['raw_bytes'] and sha(data)==item['raw_sha256']
    key=(item['case'],item['kind']);assert key not in raw;raw[key]=data
assert len(raw)==48
reported={row['case']:row for row in load('results.json')};assert len(reported)==24
identities=set();counts={};curves=[];rank6_cases=[]
metrics=('actual_run_seed','elapsed','wave','level','bosses','deaths','gate_breaks','exposed_secs','injured_secs','min_alive_health_fraction','max_health_deficit','attacks','damage_skill_casts','kills','gold','wood','actual_hero_id')
for receipt in summary['cases']:
    name=receipt['name'];policy,h,pos,seed,chapter,training=name.split('-')
    starter=int(h[1:]);requested=int(seed[1:],16)
    assert policy in ('active','passive') and pos=='behind' and chapter=='c0' and training=='t0'
    identity=(policy,starter,requested);assert identity not in identities;identities.add(identity)
    assert receipt['returncode']==0 and not receipt['timed_out']
    stderr=raw[name,'stderr'].decode();assert 'ERROR' not in stderr and 'panicked' not in stderr
    text=raw[name,'log'].decode();assert 'BALANCE_ASSET_ROOT ' in text
    records=[json.loads(line) for line in text.splitlines() if line.startswith('{')]
    runner=next(r for r in records if r.get('type')=='runner_metadata')
    metadata=next(r for r in records if r.get('type')=='metadata')
    pressure=next(r for r in records if r.get('type')=='pressure')
    assert pressure==dict(type='pressure',movement_growth=.03,attack_growth=.05,movement_cap=1.8,attack_cap=2.2,health_growth=.12,health_cap=3.7)
    assert runner['binary_sha256']==binary['sha256']
    assert metadata['chapter']==0 and metadata['training']==0 and metadata['starter']==starter
    assert metadata['seed']==requested and metadata['forward'] is False
    assert metadata['renderer'] is False and metadata['executor']=='single-threaded'
    assert metadata['frame_cap']==120000 and metadata['fixed_step_secs']==1/60
    assert metadata['policy']==('greedy-card-zero' if policy=='active' else 'passive')
    cps=[r for r in records if r.get('type')=='checkpoint'];assert cps and len(cps)==receipt['checkpoints']
    first=next(r for r in cps if r['elapsed']>0)
    assert first['actual_hero_id']=={0:2,1:1,2:0}[starter]
    previous=0.0;first_cap=None;has_six=False
    for c in cps:
        assert c['actual_chapter']==0 and c['permanent_upgrades']==[0]*6
        assert c['actual_run_seed']==first['actual_run_seed']
        assert c['damage_skill_casts'] is not None and c['elapsed']>=previous
        ranks=c['skill_ranks_by_id'];assert len(ranks)==32
        # Crown legitimately raises one skill above its base max5 to6.
        assert all(isinstance(n,int) and 0<=n<=6 for n in ranks)
        assert sum(ranks)==c['skill_ranks']
        learned=sum(n>0 for n in ranks);capped=sum(n>=m for n,m in zip(ranks,maxima))
        if capped and first_cap is None:first_cap=[previous,c['elapsed']]
        has_six |= 6 in ranks
        assert all(math.isfinite(c[k]) for k in metrics)
        curves.append(dict(case=name,policy=policy,requested_starter=starter,gate_hp=c['gate_hp'],hero_hp=c['hero_hp'],max_hero_hp=c['max_hero_hp'],deaths=c['deaths'],elapsed=c['elapsed'],level=c['level'],learned=learned,capped=capped,rank_total=sum(ranks),bonds=len(c['owned_bond_ids'])))
        previous=c['elapsed']
    last=cps[-1];assert last==receipt['last']
    outcome='win' if last['finished'] and last['victory'] else 'loss' if last['finished'] else 'input_blocked' if last['paused'] else 'frame_cap'
    censored=not bool(last['finished'])
    assert receipt['valid'] and receipt['dimension_valid'] and receipt['telemetry_valid']
    assert receipt['outcome']==outcome and receipt['censored']==censored
    # This particular sample completed every game; do not silently classify a cap as loss.
    assert not censored
    row=reported[name]
    expected=dict(case=name,policy=policy,requested_starter=starter,requested_seed_hex=seed[1:],first_live_hero_id=first['actual_hero_id'],outcome=outcome,**{k:last[k] for k in metrics},learned=learned,capped=capped,rank_total=sum(last['skill_ranks_by_id']),rank_vector=last['skill_ranks_by_id'],bonds=len(last['owned_bond_ids']),first_capped_observation_window=first_cap)
    assert row==expected,(name,row,expected)
    counts[policy,outcome]=counts.get((policy,outcome),0)+1
    if has_six:rank6_cases.append(name)
assert identities==set(itertools.product(('active','passive'),range(3),range(1,5)))
assert counts=={('active','win'):12,('passive','loss'):12}
for filename,expected in [('cases.csv',load('results.json')),('curves.csv',curves)]:
    with (D/filename).open() as f:rows=list(csv.DictReader(f))
    assert len(rows)==len(expected)
    for got,wanted in zip(rows,expected):assert got=={k:'' if v is None else str(v) for k,v in wanted.items()},(filename,got,wanted)
print(json.dumps(dict(result='PASS',raw_files=48,cases=24,uncensored=24,active_wins=12,passive_losses=12,starter_mapping='all first-positive checkpoints verified',rank6_cases=rank6_cases,scope='Descriptive fixed-behind stationary policies; not global win rate/class ranking/UR evolution proof'),indent=2))
