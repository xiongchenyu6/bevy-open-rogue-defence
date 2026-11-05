from pathlib import Path
import subprocess,json,tempfile
p=Path('/project/docs/verification/vortex-chapter6-r2-2026-11-06/chapter6/equipment-08-item-0-before.png')
for size in (100,200,300,400,600):
 for psm in (7,13):
  with tempfile.TemporaryDirectory() as d:
   out=Path(d)/'x.png';subprocess.run(['convert',str(p),'-crop','200x32+210+100','+repage','-colorspace','Gray','-negate','-resize',str(size)+'%','-bordercolor','white','-border','10',str(out)],check=True)
   raw=subprocess.check_output(['tesseract',str(out),'stdout','--psm',str(psm),'-l','eng'],stderr=subprocess.DEVNULL,text=True).strip();print(size,psm,repr(raw))
