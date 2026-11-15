#!/usr/bin/env python3
"""Original, reproducible game-control glyphs. Requires ImageMagick's SVG renderer."""
from pathlib import Path
import subprocess
root = Path(__file__).resolve().parents[1] / 'assets/ui/controls'
root.mkdir(parents=True, exist_ok=True)
shapes = {
 'lock': '<path d="M35 54V39a29 29 0 0 1 58 0v15H79V39a15 15 0 0 0-30 0v15z"/><rect x="25" y="53" width="78" height="61" rx="9"/><path d="M64 75v19" stroke="#14272b"/>',
 'play': '<path d="m36 20 69 44-69 44z"/>',
 'settings': '<path d="M53 15h22l4 15 12 7 15-4 11 19-11 11v14l11 11-11 19-15-4-12 7-4 15H53l-4-15-12-7-15 4-11-19 11-11V63L11 52l11-19 15 4 12-7z"/><circle cx="64" cy="70" r="20" fill="#14272b"/>',
 'language': '<circle cx="64" cy="64" r="43"/><ellipse cx="64" cy="64" rx="19" ry="43"/><path d="M22 64h84M30 40h68M30 88h68"/>',
 'training': '<path d="M18 45h92v15L85 76H68v21h22v12H37V97h17V76H35L18 60z"/><path d="M33 17h27v16H33zM51 33v18"/>',
 'back': '<path d="M55 25 18 64l37 39V78h55V50H55z"/>',
 'pause': '<path d="M29 23h23v82H29zM76 23h23v82H76z"/>',
 'resume': '<path d="m36 20 69 44-69 44z"/>',
 'minus': '<path d="M25 54h78v20H25z"/>',
 'plus': '<path d="M54 25h20v29h29v20H74v29H54V74H25V54h29z"/>',
 'reset': '<path d="M29 45a40 40 0 1 1-2 39" fill="none"/><path d="M15 18v35h35z"/>',
 'book': '<path d="M15 24q27-6 49 9 22-15 49-9v75q-27-6-49 9-22-15-49-9z"/><path d="M64 33v75M28 44l22 5M28 61l22 5M79 49l20-5M79 66l20-5"/>',
 'chapters': '<path d="m16 28 30-12 36 12 30-12v84l-30 12-36-12-30 12z"/><path d="M46 16v84M82 28v84M28 75l30-19 13 14 28-24"/>',
 'exit': '<path d="M56 18H23v92h33M60 64h48M88 43l21 21-21 21" fill="none"/>',
 'sound': '<path d="M16 49h23l26-23v76L39 79H16z"/><path d="M82 43q22 21 0 42M96 25q39 39 0 78" fill="none"/>',
 'fullscreen': '<path d="M17 49V17h32M79 17h32v32M111 79v32H79M49 111H17V79" fill="none"/>',
}
for name, shape in shapes.items():
    fill = 'none' if name == 'language' else '#eac783'
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><g transform="translate(0 3)" fill="{fill}" stroke="#091519" stroke-width="12" stroke-linejoin="round" stroke-linecap="round">{shape}</g><g fill="{fill}" stroke="#f5e5bc" stroke-width="5" stroke-linejoin="round" stroke-linecap="round">{shape}</g></svg>'''
    path = root / f'{name}.svg'
    path.write_text(svg)
    subprocess.run(['magick', '-background', 'none', str(path), str(path.with_suffix('.png'))], check=True)
print(f'Wrote {len(shapes)} original glyphs')
