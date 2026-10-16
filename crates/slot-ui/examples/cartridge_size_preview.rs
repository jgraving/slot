//! Export real shelf draws at RG SP resolution for a software-composited preview.
use slot_ui::{Canvas, Draw, Shelf, TexId};
use slot_store::{Cart, Platform};
use std::{fs::File, io::Write, path::{Path,PathBuf}};
fn png(dir:&Path,id:u32,w:u32,h:u32,rgba:&[u8]) {
 let mut enc=png::Encoder::new(File::create(dir.join(format!("{id}.png"))).unwrap(),w,h);
 enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);
 enc.write_header().unwrap().write_image_data(rgba).unwrap();
}
fn main(){
 let dir=PathBuf::from(std::env::args().nth(1).expect("output directory"));
 std::fs::create_dir_all(&dir).unwrap();
 let mut shelf=Shelf::new(["CLASSICS","ADVENTURE","ARCADE"].iter().map(|name|Cart {
  platform:Platform::Gba,stem:name.to_string(),rom:format!("{name}.gba").into(),label:None,
  code:String::new(),shell:None,title:name.to_string(),
 }).collect());
 for (i,cart) in shelf.carts.iter().enumerate(){let f=slot_ui::cart_face(cart);png(&dir,i as u32,f.w,f.h,&f.rgba);}
 shelf.set_faces((0..3).map(TexId::from_raw).collect());shelf.select(1);
 png(&dir,3,slot_ui::SLOT_W,slot_ui::SLOT_H,slot_ui::slot_back());
 png(&dir,4,slot_ui::SLOT_W,slot_ui::SLOT_H,slot_ui::slot_front());
 let mut draws=Vec::new();shelf.draw(Canvas{w:640.0,h:480.0},0.0,Some((TexId::from_raw(3),TexId::from_raw(4))),&mut draws);
 let mut csv=File::create(dir.join("draws.csv")).unwrap();
 writeln!(csv,"id,x,y,w,h,alpha,u,v,uw,vh").unwrap();
 for d in draws {
  let (x,y,w,h,tex,alpha,uv)=match d {
   Draw::Tex{x,y,w,h,tex,alpha}=>(x,y,w,h,tex,alpha,[0.0,0.0,1.0,1.0]),
   Draw::Part{x,y,w,h,tex,alpha,uv}=>(x,y,w,h,tex,alpha,uv), _=>continue,
  };
  let id=(0..5).find(|i|TexId::from_raw(*i)==tex).unwrap();
  writeln!(csv,"{id},{x},{y},{w},{h},{alpha},{},{},{},{}",uv[0],uv[1],uv[2],uv[3]).unwrap();
 }
}
