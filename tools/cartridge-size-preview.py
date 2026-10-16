"""Composite exported production draws; pass before-dir, after-dir, output PNG."""
import csv,sys
from pathlib import Path
from PIL import Image,ImageDraw
out=Image.new('RGB',(1280,520),'#161619')
for n,(folder,label) in enumerate(zip(sys.argv[1:3],['BEFORE','LARGER CARTRIDGES'])):
 folder=Path(folder);frame=Image.new('RGBA',(640,480),'#161619')
 for row in csv.DictReader((folder/'draws.csv').open()):
  src=Image.open(folder/(row['id']+'.png')).convert('RGBA')
  x,y,w,h,a,u,v,uw,vh=[float(row[k]) for k in ['x','y','w','h','alpha','u','v','uw','vh']]
  src=src.crop((round(u*src.width),round(v*src.height),round((u+uw)*src.width),round((v+vh)*src.height)))
  src=src.resize((round(w),round(h)),Image.Resampling.LANCZOS)
  src.putalpha(src.getchannel('A').point(lambda value:round(value*a)))
  frame.alpha_composite(src,(round(x),round(y)))
 out.paste(frame,(n*640,40));ImageDraw.Draw(out).text((n*640+24,14),label,fill='white')
Path(sys.argv[3]).parent.mkdir(parents=True,exist_ok=True)
out.save(sys.argv[3])
