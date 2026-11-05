"""Run in the same isolated OCR image with repository read-only at /repo."""
from pathlib import Path
import sys
sys.path.insert(0,'/repo/tools/desktop')
from campaign_equipment_runtime import crop_text,selected_inventory_name
from smoke_runtime import normalized
p=Path('/repo/docs/verification/inventory-selection-2026-11-06/final')
names=['Bloodreaver','Astral Prism','Storm Bastion','Suncoil','Night Crown','World Engine']
for mode in ['ordinary','held']:
 for i,name in enumerate(names):
  text=crop_text(p/f'{mode}-{i}-after.png','310x28+744+217',None)
  assert selected_inventory_name(text)==normalized(name),(mode,i,text)
  print('PASS',mode,i,repr(text),flush=True)
