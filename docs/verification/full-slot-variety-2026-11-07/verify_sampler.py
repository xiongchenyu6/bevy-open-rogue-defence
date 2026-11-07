"""Validate actual sample eligibility independently of production gate helpers."""
import argparse
from collections import Counter,defaultdict
from decimal import Decimal
import gzip
import json
from pathlib import Path
import re

def text(path):
    return gzip.decompress(path.read_bytes()).decode() if path.suffix=='.gz' else path.read_text()

def verify(log,content):
    catalog=text(content).split('pub const SKILLS:',1)[1]
    definitions=re.findall(r'SkillDef\s*\{\s*id:\s*"([^"]+)"\s*,\s*name:\s*"[^"]+"\s*,\s*rarity:\s*Rarity::(R|Sr|Ssr|Ur)\s*,.*?max_rank:\s*(\d+)',catalog,re.S)
    assert len(definitions)==32
    sample=re.compile(r'FULL_SLOT_SAMPLE level=(\d+) elapsed=([0-9.]+) remaining=(\d+) seed=(\d+) choices=(\[.*\]) rng=(\d+)')
    choice=re.compile(r'ReplaceSkill \{ old: SkillId\((\d+)\), new: SkillId\((\d+)\) \}|Skill\(SkillId\((\d+)\)\)')
    groups=defaultdict(dict);aggregate={}
    for line in text(log).splitlines():
        m=sample.fullmatch(line)
        if not m:continue
        level=int(m[1]);elapsed=Decimal(m[2]);remaining=int(m[3]);seed=int(m[4]);body=m[5][1:-1]
        key=(level,elapsed,remaining);assert seed not in groups[key]
        matches=list(choice.finditer(body));assert ', '.join(x[0] for x in matches)==body
        assert len(matches)<=3
        ranks=[int(row[2]) if i<6 else 0 for i,row in enumerate(definitions)]
        for i in range(remaining):ranks[i]=1
        ids=[];upgrades=[];new_rarities=[]
        for item in matches:
            if item[3] is not None:
                skill=int(item[3]);assert 0<=skill<6 and 0<ranks[skill]<int(definitions[skill][2])
                upgrades.append(skill)
            else:
                old,skill=int(item[1]),int(item[2]);assert 0<=old<6 and ranks[old]>0
                assert 6<=skill<32 and ranks[skill]==0
                rarity=definitions[skill][1]
                # Explicit independent boundary rules, not production helper calls.
                assert rarity!='R'
                assert (rarity=='Sr' and level>=3) or (rarity=='Ssr' and level>=8) or (rarity=='Ur' and elapsed>=720)
                new_rarities.append(rarity)
            ids.append(skill)
        assert len(set(ids))==len(ids),'whole weighted hand repeats an incoming/upgrade skill'
        assert new_rarities.count('Ur')<=1
        groups[key][seed]={'size':len(ids),'swaps':len(new_rarities),'ur':new_rarities.count('Ur'),'missing_owned':any(i not in upgrades for i in range(remaining)),'new_rarities':new_rarities,'rng_recorded':int(m[6])}
    expected={(level,Decimal(elapsed),remaining) for level,elapsed in [(2,'100'),(3,'100'),(7,'719.9'),(8,'719.9'),(8,'720'),(30,'1500')] for remaining in (0,1,2)}
    assert set(groups)==expected
    for key,rows in groups.items():
        assert set(rows)==set(range(256))
        aggregate[key]={'seeds':256,'swaps':sum(r['swaps'] for r in rows.values()),'missing_owned':sum(r['missing_owned'] for r in rows.values()),'ur':sum(r['ur'] for r in rows.values())}
    reported=re.compile(r'FULL_SLOT_DISTRIBUTION level=(\d+) elapsed=([0-9.]+) remaining=(\d+) seeds=(\d+) swaps=(\d+) missing_owned=(\d+) ur=(\d+)')
    reports={}
    for line in text(log).splitlines():
        m=reported.fullmatch(line)
        if m:
            key=(int(m[1]),Decimal(m[2]),int(m[3]));assert key not in reports
            reports[key]={'seeds':int(m[4]),'swaps':int(m[5]),'missing_owned':int(m[6]),'ur':int(m[7])}
    assert reports==aggregate
    return {'rows':sum(len(rows) for rows in groups.values()),'groups':[{'level':key[0],'elapsed':str(key[1]),'remaining_owned_upgrades':key[2],**aggregate[key],'swap_count_histogram':dict(Counter(r['swaps'] for r in groups[key].values())),'new_rarity_counts':dict(Counter(rarity for row in groups[key].values() for rarity in row['new_rarities']))} for key in sorted(groups)],'scope':'Independent catalog/gate/ownership/unique-ID verification of actual recorded hands. Same-seed repeat is asserted by the separate Rust regression test, not independently regenerated here.'}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('log',type=Path);p.add_argument('content',type=Path);args=p.parse_args()
    print(json.dumps(verify(args.log,args.content),indent=2))
