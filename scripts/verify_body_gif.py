#!/usr/bin/env python3
"""Optional independent GIF decoder check. Requires Pillow; pass a GIF path."""
import sys
from PIL import Image, ImageChops
with Image.open(sys.argv[1]) as image:
    assert image.format == 'GIF'
    frames = []
    for index in range(image.n_frames):
        image.seek(index)
        assert image.info['duration'] == 800
        frames.append(image.convert('RGB').copy())
    assert image.info['loop'] == 0
    if len(frames) > 1:
        assert any(ImageChops.difference(frames[0], frame).getbbox() for frame in frames[1:]), 'All frames identical'
    print(f'Decoded {len(frames)} frames at {image.size}, 800 ms/frame, loop forever')
