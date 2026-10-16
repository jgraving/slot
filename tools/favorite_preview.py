"""Render a geometry preview to an opaque GIF (requires Pillow).

Render favorite-star.svg with resvg at 132px to export/star.png;
The cartridge uses gba_neutral.png with an illustrative label.
python tools/favorite_preview.py /path/to/export previews/favorite-star-matte.gif
"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parents[1]
source, output = map(Path, sys.argv[1:])
cart = Image.open(root/'assets/gba_neutral.png').convert('RGBA')
d = ImageDraw.Draw(cart)
d.rounded_rectangle((48,50,312,186), radius=8, fill='#526c75')
d.text((76,109), 'FAVORITE', fill='#e5ded0')
textures = {1: cart,
            2: Image.open(source/'star.png').convert('RGBA'),
            3: Image.open(root/'assets/slot_back_dark.png').convert('RGBA'),
            4: Image.open(root/'assets/slot_front_dark.png').convert('RGBA')}
# Mirror bitmaps.rs::slot_front_with_emblem for this geometry-only preview.
for y in range(58):
    for x in range(720):
        dx,dy = (x+.5-360)/13, (y+.5-15)/14
        coverage = max(0,min(1,(1-(dx*dx+dy*dy)**.5)*13))
        if coverage > 0:
            r,g,b,a = textures[3].getpixel((x,y))
            textures[4].putpixel((x,y),(r,g,b,max(textures[4].getpixel((x,y))[3],round(coverage*a))))
# Port of slot_chrome.rs::travel/draw_with_star and favorite.rs::draw_star.
# Geometry preview only; this does not execute the Rust UI or GPU renderer.
def ease(u):
    return u*u*u*(u*(u*6-15)+10)
commands = {}
for frame in range(100):
    seat = 0 if frame < 15 or frame >= 90 else ((frame-15)/29 if frame < 45 else (1 if frame < 60 else 1-(frame-60)/29))
    catch = (422-346)/(569-346)
    travel = catch*ease(seat/.42) if seat < .42 else (catch+.03*(seat-.42)/.20 if seat < .62 else catch+.03+(1-catch-.03)*ease((seat-.62)/.38))
    k = min(travel/catch, 1)
    w,h = 360+(240-360)*k,212+(141-212)*k
    x,y = 360-w/2,346+(569-346)*travel-h
    sw = 33*w/360
    sx,sy = x+312*w/360-sw-5*w/360,y+55*h/212
    commands[frame] = [(3,0,422,720,58,1,0,0,1,1),
                       (1,x,y,w,h,1,0,0,1,1),
                       (2,sx,sy,sw,33*h/212,1,0,0,1,1),
                       (4,0,422,720,58,1,0,0,1,1)]
frames = []
for items in commands.values():
    canvas = Image.new('RGBA', (720, 480), (34, 35, 40, 255))
    for tex, x, y, w, h, alpha, u, v, uw, vh in items:
        im = textures[tex]
        im = im.crop((round(u*im.width), round(v*im.height), round((u+uw)*im.width), round((v+vh)*im.height)))
        im = im.resize((max(1, round(w)), max(1, round(h))), Image.Resampling.BILINEAR)
        if alpha != 1:
            im.putalpha(im.getchannel('A').point(lambda a: round(a*alpha)))
        canvas.alpha_composite(im, (round(x), round(y)))
    assert canvas.getchannel('A').getextrema() == (255, 255)
    draw = ImageDraw.Draw(canvas)
    draw.text((20,20), 'MATTE GOLD / INSERT + EJECT', fill='#c9ad6d')
    draw.text((20,42), 'Geometry preview - not handheld footage', fill='#aaaab0')
    assert canvas.getpixel((360,437))[:3] == textures[3].getpixel((360,15))[:3]
    frames.append(canvas.convert('RGB'))
# Full opaque frames: no GIF transparency index or disposal-dependent holes.
output.parent.mkdir(parents=True, exist_ok=True)
palette = frames[0].quantize(colors=128)
encoded = [f.quantize(palette=palette, dither=Image.Dither.NONE) for f in frames]
encoded[0].save(output, save_all=True, append_images=encoded[1:], duration=40, loop=0, disposal=1, optimize=False)
with Image.open(output) as check:
    assert 'transparency' not in check.info
    for i in range(check.n_frames):
        check.seek(i)
        assert check.convert('RGBA').getchannel('A').getextrema() == (255,255)
frames[0].save(source/'still.png')
print(f'Validated {len(frames)} source frames; GIF has no transparency.')
