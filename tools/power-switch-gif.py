#!/usr/bin/env python3
"""Assemble production poses into an opaque GIF (Pillow required).

cargo run -p slot-ui --example power_switch_preview -- /tmp/power-switch
python tools/power-switch-gif.py /tmp/power-switch previews/power-switch.gif
Software composite of exported production draws, not device footage.
"""
import csv
import sys
from pathlib import Path
from PIL import Image, ImageDraw

source, output = map(Path, sys.argv[1:])
poses = list(csv.DictReader((source / 'poses.csv').open()))
frames = []
for frame in range(31):
    image = Image.new('RGB', (720, 560), '#101013')
    draw = ImageDraw.Draw(image)
    for row, finish in enumerate(('dark', 'light')):
        draw.text((32, row * 280 + 15), f'{finish.upper()} CAROUSEL FINISH', fill='#bfc0c5')
        for pose in (p for p in poses if int(p['frame']) == frame):
            layer = Image.open(source / f"{finish}-{pose['layer']}.png").convert('RGBA')
            layer = layer.resize((round(float(pose['w'])), round(float(pose['h']))), Image.Resampling.LANCZOS)
            alpha = float(pose['alpha'])
            layer.putalpha(layer.getchannel('A').point(lambda a: round(a * alpha)))
            image.paste(layer, (round(float(pose['x'])), round(float(pose['y'])) - 95 + row * 280), layer)
    frames.append(image)
# One shared palette and fully opaque frames avoid highlight/transparency flicker.
palette = frames[0].quantize(colors=256)
frames = [f.quantize(palette=palette, dither=Image.Dither.NONE) for f in frames]
output.parent.mkdir(parents=True, exist_ok=True)
frames[0].save(output, save_all=True, append_images=frames[1:],
               duration=[800] + [20] * 29 + [700], loop=0, disposal=2, optimize=False)
print(output)
