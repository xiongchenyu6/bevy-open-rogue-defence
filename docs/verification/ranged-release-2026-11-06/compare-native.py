#!/usr/bin/env python3
"""Read-only diagnostic comparison; never a global RNG/HP/timing acceptance gate."""
import argparse
import ast
from collections import Counter
from decimal import Decimal
import hashlib
import json
import math
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parent
ANSI=re.compile(r'\x1b\[[0-9;]*m')
NUM=r'[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?'
SHA=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ORIGIN=(-2.0,.25,0.0)


def clips(raw):
    values=ast.literal_eval(raw)
    assert isinstance(values,list) and all(isinstance(v,tuple)and len(v)==3 for v in values)
    assert all(math.isfinite(float(n))for v in values for n in v)
    return values


def read(name,kind):
    path=ROOT/(name+'.log');receipt_path=ROOT/(name+'-result.json')
    assert path.is_file()and receipt_path.is_file(),('missing actual run',name)
    receipt=json.loads(receipt_path.read_text());assert receipt.get('returncode')==0,(name,receipt)
    command=receipt['command'];assert '210'in command and 'attack-'+kind in command
    text=ANSI.sub('',path.read_text())
    assert not re.search(r'ERROR|panicked|Validation Error|Caught rendering error',text),(name,'runtime error')
    assert len(re.findall(r'\[capture\] wrote 210 simulation frames to ',text))==1
    assert '[capture] PASS: real basic attack damage' in text
    footer=re.findall(r'\[capture\] all (\d+) screenshot\(s\) saved after (\d+) wait frame\(s\)',text)
    assert len(footer)==1,(name,'missing screenshot completion')
    phases=[];timing=[];releases=[];hits=[];waiting=[];headers=[]
    for ordinal,line in enumerate(text.splitlines()):
        if 'PROJECTILE_HIT' in line:
            fields={key: value for key,value in re.findall(rf'(?<!\w)(\w+)=({NUM})(?=\s|$)',line)}
            assert all(Decimal(v).is_finite()for v in fields.values())
            profile=re.search(r'\bprofile=(.*?) base_damage=',line);assert profile
            required={'base_damage','scaled_base_damage','raw_damage','final_damage','critical_roll_bits','hp_before_bits','hp_after_bits','rng_before_crit','rng_after_crit','rng_after_resolution','elapsed_bits'}
            assert required<=set(fields),('incomplete resolver telemetry',required-set(fields))
            hit=dict(log_line=ordinal,frame=None,numeric_fields=fields,profile=profile[1],raw=line)
            hits.append(hit);waiting.append(hit)
        frame=re.match(r'^(?:RELEASE_PHASE|ATTACK_TIMING|HIT_PUNCH_FIRST|HIT_MATERIAL(?:_CONTACT|_BASELINE)?) frame=(\d+)\b',line)
        if frame:
            headers.append((ordinal,int(frame[1])))
            for h in waiting:h['frame']=int(frame[1])
            waiting=[]
        m=re.fullmatch(r'RELEASE_PHASE frame=(\d+) hero=(\w+) clips=(.*)',line)
        if m:phases.append(dict(frame=int(m[1]),kind=m[2],clips=clips(m[3]),raw=line))
        m=re.fullmatch(rf'ATTACK_TIMING frame=(\d+) health=({NUM}) damage=({NUM}) clips=(.*)',line)
        if m:timing.append(dict(frame=int(m[1]),health=m[2],damage=m[3],clips=clips(m[4]),raw=line))
        m=re.fullmatch(r'PROJECTILE_RELEASE frame=(\d+) entity=(\S+) added_tick=(\d+) hero=(\w+) position=Vec3\((.*?)\) velocity=Vec3\((.*?)\) clips=(.*)',line)
        if m:
            position=[float(x)for x in m[5].split(',')];velocity=[float(x)for x in m[6].split(',')]
            assert len(position)==len(velocity)==3 and all(math.isfinite(n)for n in position+velocity)
            releases.append(dict(frame=int(m[1]),entity=m[2],added_tick=int(m[3]),kind=m[4],position=position,velocity=velocity,clips=clips(m[7]),speed=math.sqrt(sum(n*n for n in velocity)),distance_from_controlled_fixture_origin=math.sqrt(sum((x-y)**2 for x,y in zip(position,ORIGIN))),raw=line))
    assert [r['frame']for r in phases if r['frame']<=210]==list(range(1,211)),(name,'phase coverage')
    assert [r['frame']for r in timing if r['frame']<=80]==list(range(1,81)),(name,'timing coverage')
    cycles=[];previous=None
    for phase in phases:
        attack=next((c for c in phase['clips']if c[0]==3),None)
        if attack is not None and (previous is None or attack[1]<previous[1]):
            cycles.append(dict(index=len(cycles),observed_start_frame=phase['frame'],initial_seek=attack[1],initial_speed=attack[2]))
        previous=attack
    for release in releases:
        cycle=next((c for c in reversed(cycles)if c['observed_start_frame']<=release['frame']),None)
        release['observable_cycle_index']=None if cycle is None else cycle['index']
        release['updates_after_observable_cycle_start']=None if cycle is None else release['frame']-cycle['observed_start_frame']
        release['attack_clip']=next((c for c in release['clips']if c[0]==3),None)
    last_header=headers[-1][1] if headers else None
    tail=[h for h in hits if h['frame']is None]
    return dict(name=name,log_sha256=SHA(path),receipt=receipt,screenshot_completion=dict(count=int(footer[0][0]),wait_updates=int(footer[0][1])),phase_rows=phases,timing_rows=timing,animation_cycles=cycles,active_releases=releases,resolver_hits=hits,tail_observation=dict(last_available_header=last_header,phase_rows_after210=[r for r in phases if r['frame']>210],timing_rows_after80=[r for r in timing if r['frame']>80],releases_after210=[r for r in releases if r['frame']>210],unassigned_resolver_hits_after_last_header=tail,scope='No frame guessed for unassigned tail hits; screenshot wait updates can continue combat after capped phase observer.'))


def infer_phase(run,kind):
    """Conditional kinematics, never an authored hand/release or pixel event."""
    context=ROOT/'phase-source-context'
    provenance=json.loads((context/'provenance.json').read_text())
    attack_source=(context/'attacks.rs').read_text();capture_source=(context/'capture.rs').read_text()
    if run['name'].startswith('before-trace-'):
        input_map=ROOT.parent/'attack-phase-closeup-2026-11-06/inputs.json'
    elif run['name'].startswith('r3-visual-age-candidate/'):
        input_map=ROOT/'r3-visual-age-candidate/after-inputs.json'
    else:input_map=ROOT/'after-inputs.json'
    run_inputs=json.loads(input_map.read_text())
    for path,value in provenance['files'].items():
        assert SHA(context/Path(path).name)==value==run_inputs[path],('source installation mismatch',run['name'],path)
    assert 'hero.translation = Vec3::new(-2.0, 0.0, 0.0);'in attack_source
    assert 'world.remove_resource::<Autoplay>();'in attack_source
    assert 'if melee { -0.7 } else { 3.0 }'in attack_source and 'speed: 0.0' in attack_source
    assert 'world.resource_mut::<SpawnDirector>().enabled = false;'in capture_source
    assert '1.0 / 30.0' in capture_source
    assert 'attack-'+kind in run['receipt']['command']
    duration,phase=(1.0666667,.25)if kind=='ember'else(.9333333,.28)
    per_cycle=Counter(r['observable_cycle_index']for r in run['active_releases'])
    rows=[]
    for shot in run['active_releases']:
        reasons=[];position=shot['position'];velocity=shot['velocity'];speed=shot['speed']
        attack_clips=[c for c in shot['clips']if c[0]==3]
        if len(attack_clips)!=1 or len(shot['clips'])!=1:reasons.append('requires single active Graph3 Attack clip')
        if per_cycle[shot['observable_cycle_index']]!=1 or shot['observable_cycle_index']is None:reasons.append('multiple/no release-to-observable-cycle mapping; catch-up outside scope')
        displacement=[x-y for x,y in zip(position,ORIGIN)]
        travel=None;inferred=None
        if speed<=0 or velocity[0]<=0 or abs(velocity[1])+abs(velocity[2])>1e-6:reasons.append('not straight positive-X constant-velocity fixture')
        if abs(position[1]-.25)>1e-6 or abs(position[2])>1e-6 or displacement[0]<0:reasons.append('not the stationary source-confirmed origin/centerline')
        if position[0]>=2.04:reasons.append('first observation reaches controlled target contact radius; cannot exclude impact/ricochet')
        if speed>0:
            travel=shot['distance_from_controlled_fixture_origin']/speed
            if travel>1/30+1e-6:reasons.append('more than one fixed update of flight; cannot establish fresh unbounced first observation')
        if not reasons:
            seek,playback=attack_clips[0][1:]
            if playback<=0:reasons.append('nonpositive playback speed')
            else:inferred=seek-travel*playback
        rows.append(dict(frame=shot['frame'],observable_cycle_index=shot['observable_cycle_index'],position=position,velocity=velocity,speed=speed,attack_clip=shot['attack_clip'],inferred_flight_virtual_seconds=travel,inferred_source_release_seconds=inferred,regular_candidate_source_seconds=duration*phase,difference_from_regular_candidate=None if inferred is None else inferred-duration*phase,eligible=not reasons,guard_rejections=reasons))
    values=[r['inferred_source_release_seconds']for r in rows if r['eligible']]
    return dict(source_installation=dict(input_map=str(input_map),input_map_sha256=SHA(input_map),capture_files=provenance['files']),rows=rows,total_observations=len(rows),eligible_observations=len(values),spread=None if not values else dict(min=min(values),max=max(values),range=max(values)-min(values)),regular_candidate=dict(source_duration=duration,fraction=phase,source_seconds=duration*phase),assumptions=['source-confirmed hero(-2,0,0),emission+Y.25,Autoplay removed,stationary targets atX3 and spawn disabled','single observed Graph3 clip and one first-active epoch per observable cycle; multi-catchup not interpreted','positive-X collinear flight within one actual fixed1/30update and before nearest target surface: no ricochet/contact before first observation under this controlled fixture','fixed1/30 capture path assumed without optional melee-audio Automatic override; velocity/playback constant between emission and Last; distance/speed is not an instrumented emission timestamp'],limits='Derived simulation source time only, not authored hand/string event, pixel timing or latency; no tolerance/acceptance forced against candidate. Every excluded observation and reason retained.')


def phase_only():
    report=dict(status='PHASE_DIAGNOSTIC_ONLY',runs={},pending=[],scope='Old/R3/current conditional inference only; not full numeric after-trace comparison or acceptance')
    try:
        for kind in ('ember','storm'):
            for stage,name in [('old','before-trace-'+kind),('r3','r3-visual-age-candidate/'+kind),('current',kind)]:
                if not(ROOT/(name+'.log')).is_file()or not(ROOT/(name+'-result.json')).is_file():report['pending'].append(name);continue
                run=read(name,kind);report['runs'][stage+'-'+kind]=dict(log_sha256=run['log_sha256'],receipt=run['receipt'],inference=infer_phase(run,kind),tail_observation=run['tail_observation'])
    except (AssertionError,OSError,ValueError,KeyError,SyntaxError)as error:report.update(status='INVALID_EVIDENCE',error=str(error))
    output=ROOT/'phase-inference.json';output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(status=report['status'],pending=report['pending'],spreads={name:run['inference']['spread']for name,run in report['runs'].items()}),indent=2))
    return 1 if report['status']=='INVALID_EVIDENCE'else 0


def ordered_difference(a,b):
    for i in range(max(len(a),len(b))):
        left=a[i]if i<len(a)else None;right=b[i]if i<len(b)else None
        if left!=right:return dict(index=i,before=left,after=right)
    return None


def compare(a,b):
    left=[r['numeric_fields']['base_damage']for r in a['resolver_hits']]
    right=[r['numeric_fields']['base_damage']for r in b['resolver_hits']]
    shared=min(len(left),len(right))
    cycles=[]
    for i in range(max(len(a['animation_cycles']),len(b['animation_cycles']))):
        def entry(run):
            cycle=next((c for c in run['animation_cycles']if c['index']==i),None)
            first=next((r for r in run['active_releases']if r['observable_cycle_index']==i),None)
            return dict(cycle=cycle,first_active_release=first)
        x,y=entry(a),entry(b)
        cycles.append(dict(index=i,before=x,after=y,first_active_release_frame_shift=None if x['first_active_release']is None or y['first_active_release']is None else y['first_active_release']['frame']-x['first_active_release']['frame']))
    numeric=lambda r:dict(frame=r['frame'],fields={k:v for k,v in r['numeric_fields'].items()if k not in ('projectile_bits','enemy_bits')},profile=r['profile'])
    return dict(cycles=cycles,counts=dict(before_cycles=len(a['animation_cycles']),after_cycles=len(b['animation_cycles']),before_active=len(a['active_releases']),after_active=len(b['active_releases']),before_hits=len(left),after_hits=len(right)),raw_base_damage=dict(shared_hit_prefix_length=shared,shared_prefix_equal=left[:shared]==right[:shared],first_difference=ordered_difference(left,right),before_values=left,after_values=right),resolver_numeric_first_difference=ordered_difference([numeric(r)for r in a['resolver_hits']],[numeric(r)for r in b['resolver_hits']]),profiles=dict(before=sorted(set(r['profile']for r in a['resolver_hits'])),after=sorted(set(r['profile']for r in b['resolver_hits']))),scope='Differences are reported, not filtered or required equal: shifted impacts may legitimately change targets/critical RNG/HP. Profiles come from resolver logs, not a proven one-to-one release binding. Radius is not logged; do not infer it.')


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--before-only',action='store_true');parser.add_argument('--phase-only',action='store_true');args=parser.parse_args()
    if args.phase_only:return phase_only()
    report=dict(status='PENDING_AFTER',scope='Diagnostic numeric timing evidence only; no artist-calibrated release, global RNG/HP equality or physical performance claim',fixture_origin=dict(value=ORIGIN,basis='Controlled attack fixture fixed hero(-2,0,0)+Y.25 emission, not a per-update logged hero position; distance is conditional on this stationary fixture'),commit_observation='Animation graph Attack node3 seek restarts are observable cycles, not direct BasicAttackCounter/commit timestamps.',runs={},comparisons={},missing=[])
    try:
        for kind in ('ember','storm'):report['runs']['before-trace-'+kind]=read('before-trace-'+kind,kind)
        names=[('after-trace-'+kind,kind)for kind in ('ember','storm')]+[(kind,kind)for kind in ('ember','storm','sword')]
        for name,kind in names:
            if not (ROOT/(name+'.log')).is_file()or not(ROOT/(name+'-result.json')).is_file():report['missing'].append(name);continue
            report['runs'][name]=read(name,kind)
        for kind in ('ember','storm'):
            if 'after-trace-'+kind in report['runs']:
                assert report['runs']['after-trace-'+kind]['resolver_hits'],'After mode requires full numeric resolver traces'
                assert report['runs']['before-trace-'+kind]['resolver_hits']
                report['comparisons'][kind]=compare(report['runs']['before-trace-'+kind],report['runs']['after-trace-'+kind])
        if 'sword'in report['runs']:
            baseline_path=ROOT.parent/'attack-phase-closeup-2026-11-06'
            # Read identical actual log/receipt through the same parser without changing ROOT.
            current=report['runs']['sword'];text=ANSI.sub('',(baseline_path/'sword.log').read_text())
            prior=json.loads((baseline_path/'sword-result.json').read_text());assert prior['returncode']==0
            equal={header:Counter(r['raw']for r in current[field])==Counter(line for line in text.splitlines()if line.startswith(header+' '))for header,field in [('ATTACK_TIMING','timing_rows'),('RELEASE_PHASE','phase_rows')]}
            report['sword_control']=dict(baseline_log_sha256=SHA(baseline_path/'sword.log'),row_counters_equal=equal,active_carriers=len(current['active_releases']),scope='Zero Last-visible Sword carriers expected; no emitted-carrier claim.')
            assert all(equal.values())and not current['active_releases'],'Sword unchanged control differs'
        if args.before_only:report['status']='BEFORE_ONLY_NOT_AFTER_ACCEPTANCE'
        elif not report['missing']:report['status']='EVIDENCE_COMPLETE_REVIEW_DIFFERENCES'
    except (AssertionError,OSError,ValueError,KeyError,SyntaxError)as error:
        report.update(status='INVALID_EVIDENCE',error=str(error))
    output=ROOT/('before-native-analysis.json'if args.before_only else 'native-comparison.json')
    output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(status=report['status'],output=str(output),missing=report['missing'],comparisons=list(report['comparisons'])),indent=2))
    return 1 if report['status']=='INVALID_EVIDENCE'else 2 if report['status']=='PENDING_AFTER'else 0

if __name__=='__main__':raise SystemExit(main())
