from pathlib import Path
import subprocess,json,hashlib,shutil,os
r=Path.cwd();p=r/'target/verification/arrow-classification-2026-11-07'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'r2-inputs.json').write_text(json.dumps(a,indent=2));(p/'r2-library-inputs.json').write_text(json.dumps({f:h for f,h in a.items() if not f.startswith('src/bin/')},indent=2));(p/'r2-capture-inputs.json').write_text(json.dumps(a,indent=2))
for f in ['src/game/bonds.rs','src/game/combat.rs','src/game/combat/ranged_release.rs','src/game/combat/ranged_release_tests.rs','src/game/hero_fusion.rs','src/game/progression.rs','src/game/skills.rs','src/game/combat/arrow_classification_tests.rs','src/bin/capture/skill_identity.rs']:(p/('r2-'+f.replace('/','-'))).write_bytes((r/f).read_bytes())
with (p/'r2-capture-only-diff.patch').open('w') as log:subprocess.run(['git','--no-pager','diff','--no-ext-diff','--','src/bin/capture/skill_identity.rs'],stdout=log,check=True)
def run(name,cmd,env=None):
 with (p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}));assert inputs()==a,'input freeze changed';assert x.returncode==0,(name,x.returncode);print(name,'ACTUAL TERMINAL PASS',flush=True)
run('r2-focused',['nix','develop','--command','cargo','test','--locked','--release','--lib','arrow_classification_','-j2','--','--nocapture'])
run('r2-library',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2'])
run('r2-clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])
run('r2-capture-build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
exe=p/'r2-capture';shutil.copy2(r/'target/market-visual/debug/capture',exe);(p/'r2-capture.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
for size,label in [('1280x720','desktop'),('720x1280','phone')]:
 for card in [0,1]:
  name=f'r2-{label}-card{card}';env=os.environ.copy();env.update(ROGUE_CAPTURE_ARROW_CARD=str(card),ROGUE_CAPTURE_SAVE_FRAMES='120,180,240,295,330',RUST_LOG='rogue_commander::projectile_hit=trace',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
  run(name,['nix','develop','--command',str(exe),'frames',str(p/name),'360','skill-identity-7-rank-1','0xA11CE',size],env)
  text=(p/(name+'.log')).read_text();assert 'wrote 360 simulation frames' in text and 'PASS skill-identity-7' in text and 'ARROW_CARD_OBSERVATION' in text and 'PROJECTILE_HIT' in text;assert 'panicked' not in text and 'ERROR' not in text
(p/'r2-inputs-after.json').write_text(json.dumps(inputs(),indent=2));print('ARROW R2 ALL CHECKS/NATIVE TERMINAL PASS',flush=True)
