"""Composite shell_finish_preview's exported production draws into a review board."""
import csv
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

root=Path(sys.argv[1])
out=Image.new('RGB',(1472,1550),'#191d23');d=ImageDraw.Draw(out)
font=ImageFont.load_default(size=22)
for n,(key,label) in enumerate([('graphite','Graphite'),('platinum','Platinum'),('pearl_blue','Pearl Blue'),('pearl_pink','Pearl Pink')]):
    folder=root/key;frame=Image.new('RGBA',(720,480),'black')
    for r in csv.DictReader((folder/'draws.csv').open()):
        src=Image.open(folder/(r['id']+'.png')).convert('RGBA')
        x,y,w,h,a,u,v,uw,vh=[float(r[k]) for k in ['x','y','w','h','alpha','u','v','uw','vh']]
        src=src.crop((round(u*src.width),round(v*src.height),round((u+uw)*src.width),round((v+vh)*src.height)))
        src=src.resize((round(w),round(h)),Image.Resampling.LANCZOS)
        src.putalpha(src.getchannel('A').point(lambda value:round(value*a)))
        frame.alpha_composite(src,(round(x),round(y)))
    ox=16+(n%2)*736;oy=44+(n//2)*770
    out.paste(frame,(ox,oy));d.text((ox+12,oy-32),label,fill='white',font=font)
    switch=Image.new('RGBA',(720,240),(25,29,35,255))
    switch.alpha_composite(Image.open(folder/'switch-0.png'),(60,0))
    slider=Image.open(folder/'switch-1.png').resize((194,61),Image.Resampling.LANCZOS)
    switch.alpha_composite(slider,(288,84))
    out.paste(switch,(ox,oy+490))
    frame.save(folder/'carousel.png')
Path(sys.argv[2]).parent.mkdir(parents=True,exist_ok=True)
out.save(sys.argv[2])
