import sys
sys.path.insert(0,'/repo/tools/desktop')
from campaign_equipment_runtime import crop_text
from pathlib import Path
p=Path('/repo/docs/verification/inventory-split-2026-11-05/after')
for name,path,geoms in [('gear',p/'equipment-en-desktop/frame00230.png',['310x38+744+219','310x28+744+231','310x30+740+230','310x38+740+219']),('ready',p/'controller-desktop/10-forge-ready.png',['310x38+744+219','310x38+740+219','180x24+720+546','220x28+700+544','150x24+735+546'])]:
 for g in geoms:print(name,g,repr(crop_text(path,g,None)),flush=True)
