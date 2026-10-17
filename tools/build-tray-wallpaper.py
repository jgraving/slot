"""Build the quiet molded cartridge-tray wallpaper; no stickers or wood grain."""
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

root=Path(__file__).resolve().parents[1]
w,h=720,480
y,x=np.mgrid[:h,:w]
vignette=((x-360)/500)**2+((y-220)/400)**2
light=7*np.exp(-((x-270)/400)**2-((y-100)/250)**2)-9*vignette
noise=np.asarray(Image.open(root/'assets/rgsp_paint_grain.png').convert('L'),dtype=float)-128
noise=np.tile(noise,(4,6))[:h,:w]*0.40
base=np.array([79.,87.,94.])
a=np.clip(base+light[:,:,None]+noise[:,:,None],0,255).astype('uint8')
im=Image.fromarray(a).convert('RGBA')
# Work at 3x for smooth molded corners, then reduce to hardware pixels.
layer=Image.new('RGBA',(w*3,h*3));d=ImageDraw.Draw(layer)
def rounded(box,r,fill=None,outline=None,width=1):
 d.rounded_rectangle(tuple(int(c*3) for c in box),r*3,fill,outline,width*3)
# A single broad tray lip; the cartridge sits over the quiet centre.
rounded((17,16,703,448),27,outline=(18,23,29,40))
rounded((18,18,702,450),27,outline=(209,218,223,20))
# Shallow cartridge-shaped storage impression, barely visible beneath the game.
rounded((112,91,608,386),21,outline=(12,18,25,34))
rounded((113,93,607,388),21,outline=(213,222,228,17))
# Tiny moulded grip ribs and three restrained colored studs, not decals.
for k in range(3):
 rounded((39,38+k*6,63,40+k*6),1,fill=(209,219,226,24))
for k,c in enumerate([(153,174,184),(188,153,136),(177,181,155)]):
 cx=654+k*9;cy=419
 d.ellipse((cx*3,cy*3,(cx+4)*3,(cy+4)*3),fill=(*c,150))
 d.arc((cx*3,cy*3,(cx+4)*3,(cy+4)*3),0,180,fill=(10,17,24,80),width=3)
im.alpha_composite(layer.resize((w,h),Image.Resampling.LANCZOS))
im.convert('RGB').save(root/'assets/background_default.png')
im.convert('RGB').filter(ImageFilter.GaussianBlur(6)).save(root/'assets/background_default_blur.png')
