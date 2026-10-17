"""Compose current production exports into two small README previews.

cargo run -p slot-ui --example shell_finish_preview -- /tmp/readme-shell
cargo run -p slot-ui --example power_switch_preview -- /tmp/readme-switch
python3 tools/readme-previews.py /tmp/readme-shell /tmp/readme-switch previews
These are software composites of production draws, not handheld recordings.
"""
import csv
from pathlib import Path
import sys
from PIL import Image

shell, switch, output = map(Path, sys.argv[1:])
output.mkdir(parents=True, exist_ok=True)


def layer(frame, src, x, y, w, h, alpha):
    src = src.resize((max(1, round(w)), max(1, round(h))), Image.Resampling.LANCZOS)
    src.putalpha(src.getchannel('A').point(lambda value: round(value * alpha)))
    frame.alpha_composite(src, (round(x), round(y)))


frame = Image.new('RGBA', (720, 480), 'black')
folder = shell / 'graphite'
for r in csv.DictReader((folder / 'draws.csv').open()):
    src = Image.open(folder / (r['id'] + '.png')).convert('RGBA')
    x, y, w, h, a, u, v, uw, vh = [float(r[k]) for k in ['x', 'y', 'w', 'h', 'alpha', 'u', 'v', 'uw', 'vh']]
    src = src.crop((round(u * src.width), round(v * src.height), round((u + uw) * src.width), round((v + vh) * src.height)))
    layer(frame, src, x, y, w, h, a)
frame.convert('RGB').save(output / 'readme-carousel.png')

poses = list(csv.DictReader((switch / 'poses.csv').open()))
frames = []
for n in range(31):
    frame = Image.new('RGBA', (720, 480), 'black')
    for p in (p for p in poses if int(p['frame']) == n):
        # Platinum makes the switch motion legible at README thumbnail sizes.
        src = Image.open(switch / f"light-{p['layer']}.png").convert('RGBA')
        layer(frame, src, *[float(p[k]) for k in ['x', 'y', 'w', 'h', 'alpha']])
    frames.append(frame.convert('RGB'))
# Fixed palette, opaque frames and explicit disposal avoid transparency flicker.
# Preserve the actual 600ms animation, with an extra pause on black between loops.
palette = frames[0].quantize(colors=256)
frames = [f.quantize(palette=palette, dither=Image.Dither.NONE) for f in frames]
frames[0].save(output / 'readme-shutdown.gif', save_all=True, append_images=frames[1:],
               duration=[20] * 30 + [1400], loop=0, disposal=2, optimize=False)
