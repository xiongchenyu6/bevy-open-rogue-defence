"""Offline only: candidate function in memory; no helper/game files written."""
from pathlib import Path
import sys,inspect,json,ast,unittest
sys.path.insert(0,'/repo/tools/desktop');sys.path.insert(0,'/harness')
import campaign_equipment_runtime as module
image=Path('/proof/market-02-open-04.png')
raw=ast.literal_eval((Path('/proof/runtime.log').read_text()).split('Unexpected public screen while opening market: ',1)[1].strip())
before=module.public_panel_state(raw,image,None)
source=inspect.getsource(module.public_panel_state)
assert source.count(r'(?:\d+|S)')==1
candidate=source.replace(r'(?:\d+|S)',r'[A-Z0-9]+')
# Header/modal/terminal precedence and strict numeric pixel reader are unchanged.
exec(compile(candidate,'<offline-candidate>','exec'),module.__dict__)
print(json.dumps({'original_classification':before,'candidate_classification':module.public_panel_state(raw,image,None),'strict_numeric_pixels':module.battle_level_pixels(image,None),'candidate_source':candidate}),flush=True)
suite=unittest.TestLoader().discover('/repo/tools/desktop',pattern='test_*.py')
result=unittest.TextTestRunner(verbosity=2).run(suite)
raise SystemExit(0 if result.wasSuccessful()else 1)
