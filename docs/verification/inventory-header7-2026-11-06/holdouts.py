from pathlib import Path
import json,subprocess,tempfile
from campaign_equipment_runtime import inventory_header_rows,inventory_count
from smoke_runtime import ocr_screenshot
root=Path('/project/docs/verification')
paths=sorted(set(root.rglob('*inventory*.png'))|{root/'vortex-chapter6-r2-2026-11-06/chapter6/equipment-08-item-0-before.png',root/'inventory-split-2026-11-05/after/equipment-en-desktop/frame00230.png'})
for p in paths:
 with tempfile.TemporaryDirectory() as d:
  c=Path(d)/'old.png';subprocess.run(['convert',str(p),'-crop','880x65+200+80','+repage',str(c)],check=True,capture_output=True)
  old=[(t,x+200,y+80,r+200,b+80) for t,x,y,r,b in ocr_screenshot(c)]
 new=inventory_header_rows(p)
 def parsed(rows):
  try:return inventory_count(rows)
  except AssertionError:return None
 print(json.dumps(dict(path=str(p.relative_to(root)),before=parsed(old),after=parsed(new),old_titles=[r[0] for r in old if r[1]<600],isolated=[r[0] for r in new[-2:]])),flush=True)
