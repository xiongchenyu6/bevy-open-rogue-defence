"""Read-only runtime bitmap/catalog checks; no image generation or game launch."""
from pathlib import Path
import hashlib,json,re
from PIL import Image
root=Path(__file__).resolve().parents[1]
source=(root/'src/game/equipment_ui.rs').read_text()
path=re.search(r'const EMPTY_ARMORY_IMAGE: &str = "([^"]+)";',source).group(1)
image=root/'assets'/path;meta=json.loads((image.parent/'manifest.json').read_text())
assert image.name==meta['image'] and hashlib.sha256(image.read_bytes()).hexdigest()==meta['sha256']
assert image.stat().st_size==meta['bytes'] and image.stat().st_size<128*1024
im=Image.open(image);assert im.mode=='RGBA' and list(im.size)==meta['size']==[384,256]
a=im.getchannel('A');assert a.getbbox() is not None
assert a.crop((0,0,384,8)).getextrema()==(0,0)
assert a.crop((0,248,384,256)).getextrema()==(0,0)
assert a.crop((0,0,8,256)).getextrema()==(0,0)
assert a.crop((376,0,384,256)).getextrema()==(0,0)
assert a.getpixel((192,128))==0,'Rack opening must really be empty/transparent'
assert sum(a.histogram()[129:])>10000,'Nonblank rack silhouette'
print(f'PASS original empty armory: {im.width}x{im.height} RGBA, {image.stat().st_size} bytes, nonblank, transparent gutter/opening, runtime path/hash match')
