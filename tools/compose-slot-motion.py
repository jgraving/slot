"""Composite shell_finish_preview OUTPUT animation [gba|gb|gbc] production draws."""
import csv
import sys
from pathlib import Path
from PIL import Image

root=Path(sys.argv[1])/'platinum'
frames=[]
for path in sorted(root.glob('draws-*.csv')):
    frame=Image.new('RGBA',(720,480),'black')
    for r in csv.DictReader(path.open()):
        src=Image.open(root/(r['id']+'.png')).convert('RGBA')
        x,y,w,h,a,u,v,uw,vh=[float(r[k]) for k in ['x','y','w','h','alpha','u','v','uw','vh']]
        src=src.crop((round(u*src.width),round(v*src.height),round((u+uw)*src.width),round((v+vh)*src.height)))
        src=src.resize((round(w),round(h)),Image.Resampling.LANCZOS)
        src.putalpha(src.getchannel('A').point(lambda value:round(value*a)))
        frame.alpha_composite(src,(round(x),round(y)))
    frames.append(frame.convert('RGB'))
assert frames
# Same geometry in reverse for ejection; pause at either end for inspection.
frames += frames[-2:0:-1]
durations=[35]*len(frames);durations[0]=600;durations[len(frames)//2]=400
frames[0].save(sys.argv[2],save_all=True,append_images=frames[1:],duration=durations,loop=0)
# Still frames expose proportions and lip occlusion without GIF quantization.
for i in [0,10,20,30,40]:
    frames[i].save(root/f'frame-{i:03}.png')
