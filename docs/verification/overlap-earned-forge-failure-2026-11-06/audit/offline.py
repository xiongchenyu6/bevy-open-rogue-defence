from pathlib import Path
import sys,json,ast
sys.path.insert(0,'/harness')
from campaign_equipment_runtime import public_panel_state,battle_level_pixels
from smoke_runtime import ocr_screenshot
p=Path('/proof');text=(p/'runtime.log').read_text();rows=ast.literal_eval(text.split('Unexpected public screen while opening market: ',1)[1].strip());image=p/'market-02-open-04.png'
result={'actual_original_rows':rows,'original_hud_rows':[r for r in rows if r[2]<110 or 640<r[2]<700],'original_classification':public_panel_state(rows,image,None)}
try:result['fresh_strict_numeric_level_pixels']=battle_level_pixels(image,None)
except Exception as exc:result['fresh_strict_numeric_level_error']=repr(exc)
fresh=ocr_screenshot(image);result['fresh_full_ocr_rows']=fresh;result['fresh_classification']=public_panel_state(fresh,image,None)
print(json.dumps(result,indent=2))
