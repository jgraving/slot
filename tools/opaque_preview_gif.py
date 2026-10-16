"""Encode actual recorder frames without a GIF transparency index.
Usage: python tools/opaque_preview_gif.py FRAMES_DIR OUTPUT.gif
Extract first with ffmpeg -i record.mp4 -vf fps=20 FRAMES_DIR/%04d.png
"""
from pathlib import Path
import sys
from PIL import Image
frames = [Image.open(p).convert('RGB') for p in sorted(Path(sys.argv[1]).glob('*.png'))]
assert frames
# Sample the complete clip when choosing a common palette.
sample = Image.new('RGB', (frames[0].width, frames[0].height * min(12, len(frames))))
for i in range(min(12, len(frames))):
    sample.paste(frames[i * (len(frames)-1) // 11], (0, i*frames[0].height))
palette = sample.quantize(colors=192)
encoded = [f.quantize(palette=palette, dither=Image.Dither.NONE) for f in frames]
encoded[0].save(sys.argv[2], save_all=True, append_images=encoded[1:], duration=50,
                loop=0, optimize=False, disposal=1)
with Image.open(sys.argv[2]) as gif:
    assert 'transparency' not in gif.info
    for i in range(gif.n_frames):
        gif.seek(i)
        assert gif.convert('RGBA').getchannel('A').getextrema() == (255,255)
print(f'Encoded {len(frames)} opaque frames')
