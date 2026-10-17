"""Extract a seamless luminance grain from an RG SP shell photograph.

Usage: python tools/extract-shell-material.py /path/to/rgsp-blue-closeup.jpg
Reference, crop coordinates, and processing notes: MATERIALS.md.
Only an unrecognizable high-frequency surface sample is retained.
"""
import sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
from scipy.ndimage import gaussian_filter, median_filter, distance_transform_edt

ROOT = Path(__file__).resolve().parents[1]
photo = Image.open(sys.argv[1]).convert('RGB')
# Rectify a clean quadrilateral on the deck, below the D-pad and left of the speaker.
patch = photo.transform((128, 128), Image.Transform.QUAD,
                        (820, 800, 837, 941, 980, 916, 955, 781), Image.Resampling.BICUBIC)
a = np.asarray(patch, dtype=float) @ np.array([0.2126, 0.7152, 0.0722])
# Remove isolated dust glints and photographic lighting; retain real paint grain.
local = median_filter(a, size=3)
a = np.where(np.abs(a-local) > 5.0, local, a)
grain = a - gaussian_filter(a, 5.0)
# Periodic-plus-smooth decomposition (Poisson solve) removes tile-edge seams.
v = np.zeros_like(grain)
v[0, :] = grain[-1, :] - grain[0, :]; v[-1, :] = -v[0, :]
v[:, 0] += grain[:, -1] - grain[:, 0]; v[:, -1] -= grain[:, -1] - grain[:, 0]
y, x = np.meshgrid(np.arange(128), np.arange(128), indexing='ij')
den = 2*np.cos(2*np.pi*x/128) + 2*np.cos(2*np.pi*y/128) - 4
den[0, 0] = 1
smooth = np.fft.fft2(v)/den; smooth[0, 0] = 0
grain -= np.fft.ifft2(smooth).real
grain -= grain.mean(); grain *= 2.1 / grain.std()
Image.fromarray(np.uint8(np.clip(128+grain, 120, 136))).save(ROOT/'assets/rgsp_paint_grain.png')

# Geometry is retained from the existing slot masks, not its leather-like lighting.
for front in [False, True]:
    src = np.asarray(Image.open(ROOT/f'assets/slot_{"front" if front else "back"}_light.png').convert('RGBA'))
    alpha = src[:, :, 3]; h, w = alpha.shape
    y, x = np.mgrid[:h, :w]
    relief = 3.0 - y*0.10 + 3.0*np.exp(-((x-w*.35)/(w*.6))**2)
    if front:
        field = gaussian_filter((alpha > 128).astype(float), 0.65)
        gy, gx = np.gradient(field)
        relief += (gy-gx*.25)*60.0
        # Preserve a readable rolled lip around the original thumb-scoop mask.
        distance = distance_transform_edt(alpha > 128)
        relief -= 10.0*np.exp(-distance/2.2)
        # A slim highlight on the upper painted edge, restrained satin sheen.
        relief += 8.0*np.exp(-((y-1.1)/1.1)**2)
        painted = np.ones((h,w),dtype=bool)
    else:
        # Recover the original rounded insertion lip's broad cross-section.
        # Median across a hardware-free strip rejects the old pebbled grain.
        luma = src[:, :, :3].astype(float) @ np.array([0.2126, 0.7152, 0.0722])
        profile = gaussian_filter(np.median(luma[:, 35:205], axis=1), 0.65)
        relief = (profile[:, None]-np.median(profile[5:20]))*0.88 + np.zeros((h,w))
        relief += 3.0*np.exp(-((x-w*.35)/(w*.6))**2)
        # The two small U-shaped insertion fasteners are geometry, not grain.
        # Retain their photograph-derived relief separately from the paint.
        detail = gaussian_filter(luma, 0.55)-gaussian_filter(luma, (0.55, 10.0))
        for cx in [300, 420]:
            fade = np.clip((14.0-np.abs(x-cx))/3.0, 0.0, 1.0)
            fade *= np.clip((28.0-y)/3.0, 0.0, 1.0)
            relief += detail*fade*1.10
        painted = y < 29
    out = np.zeros_like(src)
    out[:, :, 0] = np.where(painted,np.clip(128+relief,0,255),np.clip(9+(y-29)*.10,0,255))
    out[:, :, 1] = painted*255
    out[:, :, 3] = alpha
    Image.fromarray(out).save(ROOT/f'assets/slot_{"front" if front else "back"}_material.png')
print('Extracted 128x128 seamless grain and neutral slot geometry.')
