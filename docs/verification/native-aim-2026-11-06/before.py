import pathlib,subprocess
out=pathlib.Path('target/verification/native-aim-2026-11-06')
with (out/'before-tests.log').open('w') as log:p=subprocess.run(['nix','develop','--command','cargo','test','--locked','--release','--lib','native_','-j2','--','--nocapture'],stdout=log,stderr=subprocess.STDOUT)
(out/'before-exit.txt').write_text(str(p.returncode)+'\n');print('BEFORE terminal exit',p.returncode,flush=True)
assert p.returncode!=0
text=(out/'before-tests.log').read_text();assert 'equal_front_native_attacks_prefer_closest_visible_target ... FAILED' in text and 'equal_front_symmetric_native_aim_uses_visible_coordinate_tie ... FAILED' in text
assert 'native_front_priority_still_beats_a_closer_enemy_behind_it ... ok' in text
