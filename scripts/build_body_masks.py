#!/usr/bin/env python3
"""Rebuild embedded region masks from the upstream SVG paths.
Requires Python Pillow and the rsvg-convert executable (librsvg).
Only needed when editing the SVG artwork; cargo builds need neither tool.
"""
import io
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET
from PIL import Image

ROOT = Path(__file__).resolve().parents[1] / 'src/assets/body-map'
regions = json.loads((ROOT / 'regions.json').read_text())
geometry = {}
for side in ['front', 'back']:
    svg = ET.parse(ROOT / f'{side}.svg').getroot()
    for path in svg.findall('{http://www.w3.org/2000/svg}path'):
        geometry[int(path.attrib['id'].removeprefix('region-'))] = path.attrib['d']
assert len(geometry) == len(regions) == 73
masks = bytearray()
for region in regions:
    region.pop('d', None)  # Geometry lives in the SVG files.
    svg = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 375.42 832.97" width="376" height="833"><path fill="white" d="{geometry[region["id"]]}"/></svg>'
    png = subprocess.run(['rsvg-convert'], input=svg.encode(), capture_output=True, check=True).stdout
    alpha = Image.open(io.BytesIO(png)).convert('RGBA').getchannel('A')
    left, top, right, bottom = alpha.getbbox()
    crop = alpha.crop((left, top, right, bottom))
    interior = [(x + left, y + top) for y in range(crop.height) for x in range(crop.width) if crop.getpixel((x,y)) > 128]
    cx = sum(x for x,y in interior) / len(interior)
    cy = sum(y for x,y in interior) / len(interior)
    x,y = min(interior, key=lambda p: (p[0]-cx)**2+(p[1]-cy)**2)
    region.update(bounds=[left, top, crop.width, crop.height], center=[(x+0.5)/376, (y+0.5)/833], offset=len(masks))
    masks.extend(crop.tobytes())
(ROOT / 'masks.bin').write_bytes(masks)
(ROOT / 'regions.json').write_text(json.dumps(regions, ensure_ascii=False, indent=2) + '\n')
print(f'Generated {len(regions)} regions, {len(masks)} bytes of cropped alpha masks')
