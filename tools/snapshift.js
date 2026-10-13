// node tools/snapshift.js harnessdir name rsdir shotrow snaprow offset: per shot, the horizontal shift (px) between the shot and snap k=F-F0+offset that fits best in a BG band, and the residual
const fs=require('fs');const {readPng}=require('./pngcmp.js');
function readPpm(file){const b=fs.readFileSync(file);let p=0;const tok=()=>{while(b[p]===10||b[p]===32||b[p]===13)p++;let s='';while(b[p]!==10&&b[p]!==32&&b[p]!==13)s+=String.fromCharCode(b[p++]);return s;};tok();const w=+tok(),h=+tok();tok();p++;return {w,h,rgb:b.slice(p,p+w*h*3)};}
const [dir,name,rs,srow,prow,off]=process.argv.slice(2,8);
const map=fs.readFileSync(`${dir}/shmap_${name}.txt`,'utf8').trim().split('\n').map(l=>l.split(' ').map(Number)).filter(x=>x[1]===+srow);
const f0=map[0][0];
for(const [f] of map){const k=f-f0+ +off;const fn=`${rs}/rs_${prow}_${k}.ppm`;if(!fs.existsSync(fn))continue;if((f-f0)%3)continue;
 const a=readPng(`${dir}/shot_${name}_${f}.png`),b=readPpm(fn);let best=[1e9,0];
 for(let dx=-24;dx<=24;dx++){let c=0;for(let y=60;y<180;y++)for(let x=40;x<216;x++){const i=(y*256+x)*3,j=(y*256+x+dx)*3;if(a.rgb[i]!==b.rgb[j]||a.rgb[i+1]!==b.rgb[j+1])c++;}if(c<best[0])best=[c,dx];}
 console.log(`${f} k=${k} best dx ${best[1]} residual ${best[0]}`);}
