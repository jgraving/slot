"""Composite layout_review's production draws. ROOT OUTPUT.png [menu|link|sound|ports]."""
from pathlib import Path
import csv,sys,math
from PIL import Image,ImageDraw,ImageFont
root=Path(sys.argv[1]);kind=sys.argv[3] if len(sys.argv)>3 else 'menu'
names=['menu-general','menu-general-bottom','menu-game-top','menu-game-bottom'] if kind=='menu' else ['link-cable-pick','link-cable-seated','link-wireless-seated','link-cable-unplug']
if kind=='sound':names=['sound-720','sound-640']
if kind=='ports':
 board=Image.new('RGB',(1040,4*146),'#191d23');d=ImageDraw.Draw(board);font=ImageFont.load_default(size=20)
 for n,p in enumerate(['graphite','platinum','pearl_blue','pearl_pink']):
  y=n*146;im=Image.open(root/f'port-{p}/0.png').convert('RGB')
  d.text((16,y+8),p.replace('_',' ').title()+' - native strip and 2x marking detail',font=font,fill='white')
  board.paste(im,(0,y+40));board.paste(im.crop((280,0,440,58)).resize((320,116),Image.Resampling.NEAREST),(720,y+30))
 board.save(sys.argv[2]);sys.exit()
board=Image.new('RGB',(1440,520 if kind=='sound' else 1040),'#191d23');d=ImageDraw.Draw(board);font=ImageFont.load_default(size=20)
for n,name in enumerate(names):
 folder=root/name;frame=Image.new('RGBA',(720,480),'black')
 for r in csv.DictReader((folder/'draws.csv').open()):
  src=Image.open(folder/(r['id']+'.png')).convert('RGBA')
  x,y,w,h,a,u,v,uw,vh,turn=[float(r[k]) for k in ['x','y','w','h','alpha','u','v','uw','vh','turn']]
  src=src.crop((round(u*src.width),round(v*src.height),round((u+uw)*src.width),round((v+vh)*src.height))).resize((max(1,round(w)),max(1,round(h))),Image.Resampling.LANCZOS)
  src.putalpha(src.getchannel('A').point(lambda value:round(value*a)))
  if turn:
   old=src.size;src=src.rotate(-math.degrees(turn),resample=Image.Resampling.BICUBIC,expand=True);x-=(src.width-old[0])/2;y-=(src.height-old[1])/2
  frame.alpha_composite(src,(round(x),round(y)))
 ox=(n%2)*720;oy=(n//2)*520
 d.text((ox+16,oy+8),name.replace('-',' ').title(),font=font,fill='white');board.paste(frame,(ox,oy+40));frame.save(folder/'frame.png')
board.save(sys.argv[2])
