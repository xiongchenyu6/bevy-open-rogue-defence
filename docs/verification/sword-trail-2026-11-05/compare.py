import pathlib,re,json
root=pathlib.Path(__file__).resolve().parent
result={}
for scenario in ('attack-sword','attack-solar','weapon-growth-sword-upgraded','attack-ember'):
    before=root/('before-upgraded-full' if scenario=='weapon-growth-sword-upgraded' else 'before')/(scenario+'.log')
    after=root/'after'/(scenario+'.log')
    b=before.read_text();a=after.read_text()
    assert 'panicked at' not in a and 'wrote 250 simulation frames' in a
    traces=lambda text:re.findall(r'^ATTACK_TIMING .+$',text,re.M)
    assert len(traces(b))==80 and traces(b)==traces(a),(scenario,'damage/playback changed')
    rows=re.findall(r'SWORD_TRAIL frame=(\d+) entity=(\S+) alpha=([0-9.]+) mode=(\S+) material_count=(\d+)',a)
    item={'damage_and_playback_80_frames_identical':True,'after_completed_250':True,'trail_samples':len(rows)}
    if scenario in ('attack-sword','attack-solar'):
        by={}
        for frame,entity,alpha,mode,count in rows:
            assert mode=='Blend' and 0<float(alpha)<=.78
            by.setdefault(entity,[]).append(float(alpha))
        assert by and any(len(set(v))>=3 for v in by.values()),'need actual multi-phase fading'
        assert all(all(x>=y for x,y in zip(v,v[1:])) for v in by.values()),'existing slash brightened'
        item['actual_alpha_range']=[min(float(r[2]) for r in rows),max(float(r[2]) for r in rows)]
        item['per_entity_fade_monotonic']=True
    elif scenario=='weapon-growth-sword-upgraded':
        assert rows and all(r[3]=='Opaque' and float(r[2])==1 for r in rows)
        assert 'PASS weapon-growth-sword-upgraded' in a
        item['advanced_material_unchanged']=True
    else:assert not rows
    result[scenario]=item
(root/'comparison.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
