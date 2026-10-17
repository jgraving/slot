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

print('Extracted 128x128 seamless paint grain.')
