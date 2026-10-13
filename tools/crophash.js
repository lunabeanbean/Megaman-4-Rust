// node tools/crophash.js shotsdir ppmdir x0 y0 w h from to [off=7]: which crop of ours equals which crop of Mesen (find frame offsets)
const fs=require('fs');const {readPng}=require('./pngcmp.js');
function readPpm(file){const b=fs.readFileSync(file);let p=0;const tok=()=>{while(b[p]===10||b[p]===32||b[p]===13)p++;let s='';while(b[p]!==10&&b[p]!==32&&b[p]!==13)s+=String.fromCharCode(b[p++]);return s;};tok();const w=+tok(),h=+tok();tok();p++;return {w,h,rgb:b.slice(p,p+w*h*3)};}
const [a,b,x0,y0,w,h,f0,f1]=[process.argv[2],process.argv[3],+process.argv[4],+process.argv[5],+process.argv[6],+process.argv[7],+process.argv[8],+process.argv[9]];
const off=+(process.argv[10]||7);
function crop(img,W){const o=[];for(let y=y0;y<y0+h;y++)for(let x=x0;x<x0+w;x++){const i=(y*W+x)*3;o.push(img.rgb[i],img.rgb[i+1],img.rgb[i+2]);}return Buffer.from(o).toString('base64');}
const M={},R={};
for(let f=f0;f<=f1;f++){try{M[f]=crop(readPng(`${a}/sh_${f}.png`),256);}catch(e){}try{R[f]=crop(readPpm(`${b}/rs_${f-off}.ppm`),256);}catch(e){}}
for(let f=f0;f<=f1;f++){const same=[];for(let g=f0;g<=f1;g++)if(M[f]&&R[g-0]&&M[f]===R[g])same.push(g-off);console.log('mesen',f,'equals rust flow frame(s)',same.join(',')||'none');}
