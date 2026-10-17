//! Export production menu and link draw commands for software-composited visual review.
use slot_ui::*;
use slot_store::ShellColour;
use slot::app::{GameMenu,LinkRow};
use slot::link_kind::LinkKind;
use slot::link_screen::{LinkSprites,Sprite,draw_link_art};
use std::{path::{Path,PathBuf},fs::File,io::Write};

struct Scene {dir:PathBuf, ids:Vec<TexId>}
impl Scene {
    fn new(root:&Path,name:&str)->Self {let dir=root.join(name);std::fs::create_dir_all(&dir).unwrap();Self{dir,ids:vec![]}}
    fn add(&mut self,rgba:&[u8],w:u32,h:u32)->TexId {
        let n=self.ids.len();let id=TexId::from_raw(n);self.ids.push(id);
        let mut e=png::Encoder::new(File::create(self.dir.join(format!("{n}.png"))).unwrap(),w,h);
        e.set_color(png::ColorType::Rgba);e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(rgba).unwrap();id
    }
    fn hint(&mut self,f:UndoFace)->(TexId,u32){(self.add(&f.rgba,f.w,f.h),f.w)}
    fn face(&mut self,f:UndoFace)->(TexId,u32,u32){(self.add(&f.rgba,f.w,f.h),f.w,f.h)}
    fn sprite(&mut self,f:CartFace)->Sprite {Sprite{tex:self.add(&f.rgba,f.w,f.h),w:f.w,h:f.h}}
    fn save(&mut self,draws:Vec<Draw>){
        let mut csv=File::create(self.dir.join("draws.csv")).unwrap();
        writeln!(csv,"id,x,y,w,h,alpha,u,v,uw,vh,turn").unwrap();
        for d in draws {
            let (x,y,w,h,tex,alpha,uv,turn)=match d {
                Draw::Tex{x,y,w,h,tex,alpha}=>(x,y,w,h,tex,alpha,[0.,0.,1.,1.],0.),
                Draw::Part{x,y,w,h,tex,alpha,uv}=>(x,y,w,h,tex,alpha,uv,0.),
                Draw::Turned{x,y,w,h,tex,alpha,turn}=>(x,y,w,h,tex,alpha,[0.,0.,1.,1.],turn),
                Draw::Rect{x,y,w,h,colour}=>{let tex=self.add(&colour.map(|v|(v*255.) as u8),1,1);(x,y,w,h,tex,1.,[0.,0.,1.,1.],0.)},
                _=>continue,
            };
            let id=self.ids.iter().position(|id|*id==tex).unwrap();
            writeln!(csv,"{id},{x},{y},{w},{h},{alpha},{},{},{},{},{turn}",uv[0],uv[1],uv[2],uv[3]).unwrap();
        }
    }
}
fn main(){
 let root=PathBuf::from(std::env::args().nth(1).expect("output directory"));
 for (name,row) in [("menu-general",QuickRow::ShellColour),("menu-general-bottom",QuickRow::DateTime),("menu-game-top",QuickRow::FastForward),("menu-game-bottom",QuickRow::EjectSave)] {
  let mut s=Scene::new(&root,name);
  let faces=QuickMenuFaces {
   labels:QuickRow::ALL.map(|r|[s.face(quick_label_face(r,false)),s.face(quick_label_face(r,true))]).to_vec(),
   values:QuickValue::ALL.map(|v|[s.face(quick_value_face(v.text(),false)),s.face(quick_value_face(v.text(),true))]).to_vec(),
   pointer:s.face(quick_pointer_face()),carets:[s.face(quick_caret_face(false)),s.face(quick_caret_face(true))],
   legend:quick_legend_faces(footer_ink(false)).map(|f|s.hint(f)),
   legend_light:quick_legend_faces(footer_ink(true)).map(|f|s.hint(f)),
  };
  let bg=s.add(default_background().unwrap(),720,480);
  let [back,front]=shell_slot_faces(ShellColour::Platinum);
  let slot=Some((s.add(back,720,SLOT_H),s.add(front,720,SLOT_H)));
  let mut out=vec![];
  let mut values=[None;QuickRow::ALL.len()];values[QuickRow::ShellColour.index()]=Some(QuickValue::PearlBlue);
  values[QuickRow::Theme.index()]=Some(QuickValue::Dark);values[QuickRow::Recents.index()]=Some(QuickValue::On);
  values[QuickRow::ShowTitles.index()]=Some(QuickValue::Show);values[QuickRow::FastForward.index()]=Some(QuickValue::Speed3);
  values[QuickRow::FastForwardSound.index()]=Some(QuickValue::On);values[QuickRow::Rumble.index()]=Some(QuickValue::On);
  QuickMenu{row,hidden:&[],values,clock:None,wallpaper:None,faces:Some(&faces),ground:Some(bg),palette:None,
   gba_bezel:None,gb_bezel:None,gbc_bezel:None,bezel:None,over_game:false,footer_legends:true,footer_top:480.,canvas:Canvas::FRAMED}.draw(&mut out);
  draw_empty_slot(Canvas::FRAMED,slot,&mut out);
  for (i,(tex,w)) in [faces.legend[0],faces.legend[if row.opens(){2}else{1}]].into_iter().enumerate(){
   out.push(Draw::Tex{x:if i==0{16.}else{720.-16.-w as f32+HINT_EDGE as f32},y:480.-MOUTH_H-MENU_HINT_H as f32-12.,w:w as f32,h:MENU_HINT_H as f32,tex,alpha:1.});
  }
  s.save(out);
 }
 for (colour,face) in ShellColour::FINISHES.into_iter().zip(link_art().port_finishes) {
  let mut s=Scene::new(&root,&format!("port-{}",colour.key()));
  s.add(&face.rgba,face.w,face.h);
 }
 for w in [640,720] {
  let mut s=Scene::new(&root,&format!("sound-{w}"));
  let canvas=Canvas{w:w as f32,h:480.};
  let lines=["","","COMPACT","ROW SPEAKER_DEVICE Internal speaker","SELECTED ACTIVE DEVICE WF-C510",
   "ROW USB_DEVICE USB headphones","ROW Pair audio device","ROW DISCONNECT 0%","ROW BLUETOOTH On","B Back · A Connect · Y Forget"].map(str::to_owned);
  let f=slot::bluetooth::screen_face_on(&lines,footer_ink(false),canvas);let tex=s.add(&f.rgba,f.w,f.h);
  let [back,front]=shell_slot_faces(ShellColour::Platinum);let slot=Some((s.add(back,720,SLOT_H),s.add(front,720,SLOT_H)));
  let mut out=vec![Draw::Rect{x:0.,y:0.,w:w as f32,h:480.,colour:[0.08,0.10,0.11,1.]}];
  draw_empty_slot(canvas,slot,&mut out);out.push(Draw::Tex{x:0.,y:0.,w:w as f32,h:480.,tex,alpha:1.});s.save(out);
 }
 for (name,kind,menu,now) in [
  ("link-cable-pick",LinkKind::Cable,GameMenu::Pick(LinkRow::Host),0),
  ("link-cable-seated",LinkKind::Cable,GameMenu::Linked{role:LinkRow::Host,worked:0,since:200,opened:true},400),
  ("link-wireless-seated",LinkKind::Wireless,GameMenu::Linked{role:LinkRow::Join,worked:0,since:200,opened:true},400),
  ("link-cable-unplug",LinkKind::Cable,GameMenu::Unplug{role:LinkRow::Host,since:400},530),
 ]{
  let mut s=Scene::new(&root,name);let a=link_art();
  let sprites=LinkSprites{port:s.sprite(a.port_light),plug_host:s.sprite(a.plug_host),plug_join:s.sprite(a.plug_join),adapter:s.sprite(a.adapter),
   arcs_right:a.arcs_right.map(|f|s.sprite(f)),arcs_left:a.arcs_left.map(|f|s.sprite(f)),clicks:s.sprite(a.clicks),arrow_left:s.sprite(a.arrow_left),arrow_right:s.sprite(a.arrow_right)};
  let mut out=vec![Draw::Rect{x:0.,y:0.,w:720.,h:480.,colour:[0.12,0.15,0.17,1.]}];
  draw_link_art(menu,kind,now,&sprites,&mut out);s.save(out);
 }
}
