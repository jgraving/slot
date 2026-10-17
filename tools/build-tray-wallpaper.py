"""A plain matte workmat: soft light and fine grain, without tray outlines or studs."""
from pathlib import Path
import numpy as np
from PIL import Image, ImageFilter

root = Path(__file__).resolve().parents[1]
w, h = 720, 480
y, x = np.mgrid[:h, :w]
# Muted petrol surface; the cartridges and selected shell supply the playful color.
light = 5*np.exp(-((x-310)/520)**2-((y-170)/380)**2)
light -= 5*(((x-360)/600)**2+((y-240)/500)**2)
grain = np.asarray(Image.open(root/'assets/rgsp_paint_grain.png').convert('L'), dtype=float)-128
grain = np.tile(grain, (4, 6))[:h, :w]*.20
rgb = np.clip(np.array([62., 77., 81.])+light[:,:,None]+grain[:,:,None], 0, 255).astype('uint8')
im = Image.fromarray(rgb)
im.save(root/'assets/background_default.png')
im.filter(ImageFilter.GaussianBlur(6)).save(root/'assets/background_default_blur.png')
