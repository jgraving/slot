"""Compose production motion stills: INPUT_ROOT OUTPUT.png.

INPUT_ROOT contains motion-gba, motion-gb and motion-gbc from the documented
shell_finish_preview exporter and compose-slot-motion.py.
"""
from pathlib import Path
import sys
from PIL import Image, ImageDraw, ImageFont

root=Path(sys.argv[1])
board=Image.new('RGB',(2160,1040),'#191d23')
draw=ImageDraw.Draw(board)
font=ImageFont.load_default(size=22)
for col,platform in enumerate(['gba','gb','gbc']):
    for row,frame in enumerate([0,40]):
        y=row*520
        draw.text((col*720+16,y+8),f'{platform.upper()} - '+['Carousel','Seated'][row],font=font,fill='white')
        board.paste(Image.open(root/f'motion-{platform}/platinum/frame-{frame:03}.png'),(col*720,y+40))
board.save(sys.argv[2])
