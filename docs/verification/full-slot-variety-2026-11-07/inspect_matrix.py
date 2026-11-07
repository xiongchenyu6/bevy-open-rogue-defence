"""Independent raw-case recomputation for the frozen first-chapter sample."""
import argparse
from collections import Counter
import gzip
import json
from pathlib import Path
import re

RARITY = {'R':0,'SR':1,'SSR':2,'UR':3}
def read(path):
    if path.exists():return path.read_text()
    return gzip.decompress(path.with_suffix(path.suffix+'.gz').read_bytes()).decode()
def inspect(summary_path, expected):
    summary=json.loads(summary_path.read_text())
    assert len(summary['cases'])==expected
    identities=set();rows=[]
    for case in summary['cases']:
        match=re.fullmatch(r'(active|passive)-h([012])-behind-s([0-9a-f]+)-c0-t0',case['name']);assert match
        policy,starter,seed=match.groups();starter=int(starter)
        assert (policy,starter,seed) not in identities;identities.add((policy,starter,seed))
        text=read(summary_path.parent/Path(case['log']).name)
        stderr=read(summary_path.parent/Path(case['stderr']).name)
        assert not case['timed_out'] and case['returncode']==0 and case['valid']
        assert 'ERROR' not in stderr and 'panicked' not in stderr
        records=[]
        for line in text.splitlines():
            if line.startswith('{'):records.append(json.loads(line))
        metadata=next(r for r in records if r.get('type')=='metadata')
        assert metadata['seed']==int(seed,16) and metadata['starter']==starter and metadata['policy']==('greedy-card-zero' if policy=='active' else 'passive')
        checkpoints=[r for r in records if r.get('type')=='checkpoint'];assert checkpoints
        assert checkpoints[-1]==case['last']
        actual_seed=(int(seed,16)+1)%(2**64)
        assert all(r['actual_run_seed']==actual_seed for r in checkpoints)
        first=next(r for r in checkpoints if r['elapsed']>0)
        assert first['actual_hero_id']=={0:2,1:1,2:0}[starter]
        last=checkpoints[-1]
        assert last['finished'] and not case.get('censored',False)
        assert all(r['actual_chapter']==0 and r['permanent_upgrades']==[0]*6 for r in checkpoints)
        assert all(len(r['skill_ranks_by_id'])==32 and all(0<=rank<=6 for rank in r['skill_ranks_by_id']) for r in checkpoints)
        offers={};offer_sizes=Counter();swap_counts=Counter();offer_new_rarities=Counter();violations=[]
        for offer in [r for r in records if r.get('type')=='draft_offer']:
            assert offer['root'] not in offers;offers[offer['root']]=offer
            choices=offer['choices'];assert [c['index'] for c in choices]==list(range(len(choices)))
            replacements=[c for c in choices if c['kind']=='replace_skill']
            offer_sizes[len(choices)]+=1;swap_counts[len(replacements)]+=1
            offer_new_rarities.update(c['new_rarity'] for c in replacements)
            if len({c['new'] for c in replacements})!=len(replacements):violations.append({'root':offer['root'],'kind':'repeated_incoming_skill'})
            weighted=[c for c in choices if c['kind'] in ('skill','replace_skill')]
            if sum(c['rarity']=='UR' for c in weighted)>1:violations.append({'root':offer['root'],'kind':'multiple_ur_weighted_candidates'})
        swaps=[];failed=0
        for event in [r for r in records if r.get('type')=='draft_settlement']:
            assert event['root'] in offers
            offer=offers[event['root']];assert offer['frame']<=event['frame'] and event['selected_index']==0
            assert event['request_policy_frame']==event['policy_frame']
            selected=offer['choices'][0]
            if not event['root_retired']:failed+=1;continue
            if selected['kind']!='replace_skill':continue
            replacement=event['replacement'];assert replacement
            assert (replacement['old'],replacement['new'])==(selected['old'],selected['new'])
            assert replacement['old_base_rank_after']==0 and replacement['new_base_rank_after']==1
            assert replacement['old_base_rank_before']>0 and replacement['new_base_rank_before']==0
            swaps.append({'frame':event['frame'],'elapsed':event['elapsed'],'old':selected['old'],'new':selected['new'],'old_rarity':selected['old_rarity'],'new_rarity':selected['new_rarity'],'old_base_rank':replacement['old_base_rank_before'],'lower_rarity':RARITY[selected['new_rarity']]<RARITY[selected['old_rarity']],'rank_reset':replacement['old_base_rank_before']>1,'funds_before':event['funds_before'],'funds_after':event['funds_after']})
        rows.append({'case':case['name'],'policy':policy,'requested_starter':starter,'requested_seed_hex':seed,'metadata_requested_seed':metadata['seed'],'first_live_hero':first['actual_hero_id'],'actual_run_seed':first['actual_run_seed'],'outcome':'win' if last['victory'] else 'loss','terminal_elapsed':last['elapsed'],'terminal_wave':last['wave'],'deaths':last['deaths'],'rank_vector':last['skill_ranks_by_id'],'offers':len(offers),'offer_sizes':dict(offer_sizes),'swap_candidates_per_offer':dict(swap_counts),'offered_new_rarities':dict(offer_new_rarities),'settled_swaps':swaps,'lower_rarity_swaps':sum(r['lower_rarity'] for r in swaps),'rank_resets':sum(r['rank_reset'] for r in swaps),'unretired_requests':failed,'new_pool_contract_violations':violations})
    return {'cases':sorted(rows,key=lambda r:r['case']),'scope':'Paired fixed-policy descriptive fixtures. Passive includes compulsory recruitment. Candidate violations retained for historical baseline and assessed against final source only. Crown raw rank6 is valid.'}
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('summary',type=Path);parser.add_argument('--expected-cases',type=int,default=12);args=parser.parse_args()
    print(json.dumps(inspect(args.summary,args.expected_cases),indent=2))
