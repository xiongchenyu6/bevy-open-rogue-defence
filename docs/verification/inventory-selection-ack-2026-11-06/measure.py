from pathlib import Path
import subprocess,json
root=Path('/repo');files=set()
for pattern in ['target/verification/equipment-mouse*/en-desktop/*.png','target/verification/inventory-split*/equipment-en-desktop/*.png','docs/verification/inventory-chapter6-2026-11-06/chapter6/equipment-*-inventory*.png','docs/verification/inventory-split-2026-11-05/after/equipment-en-desktop/*.png','docs/verification/equipment-mouse-2026-11-06/en-desktop/frame00[23]*.png','docs/verification/inventory-close-2026-11-06/earned-failure/forge/current-ui.png','target/verification/inventory-earned-forge-r2-2026-11-06/forge/*icon*.png','target/verification/inventory-earned-forge-r2-2026-11-06/forge/inventory-material*.png']:
 files.update(root.glob(pattern))
rows=[]
for image in sorted(files):
 pts=[]
 for i in range(5):
  x=216+104*(i%3);y=148+104*(i//3)
  raw=subprocess.check_output(['convert',str(image),'-crop',f'1x1+{x}+{y}','+repage','RGB:-'])
  pts.append(list(raw))
 rows.append({'image':str(image.relative_to(root)),'pixels':pts})
print(json.dumps(rows,indent=2))
