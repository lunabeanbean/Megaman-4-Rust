// node tools/snapcmp.js harnessdir name rsdir row [dmin dmax]: the shots of one logic row (many video frames inside a scroll loop) against the per-iteration pictures rs_<row>_<k>.ppm
const fs=require('fs');const {readPng}=require('./pngcmp.js');
function readPpm(file){const b=fs.readFileSync(file);let p=0;const tok=()=>{while(b[p]===10||b[p]===32||b[p]===13)p++;let s='';while(b[p]!==10&&b[p]!==32&&b[p]!==13)s+=String.fromCharCode(b[p++]);return s;};tok();const w=+tok(),h=+tok();tok();p++;return {w,h,rgb:b.slice(p,p+w*h*3)};}
const [dir,name,rs,rowS]=process.argv.slice(2,6);const row=+rowS;const dmin=+(process.argv[6]||-3),dmax=+(process.argv[7]||3);
const map=fs.readFileSync(`${dir}/shmap_${name}.txt`,'utf8').trim().split('\n').map(l=>l.split(' ').map(Number)).filter(x=>x[1]===row);
const f0=map[0][0];const out=[];
for(const [f] of map){const k=f-f0;const a=readPng(`${dir}/shot_${name}_${f}.png`);const res=[];
 for(let d=dmin;d<=dmax;d++){const fn=`${rs}/rs_${+(process.env.SNAPROW||row)}_${k+d}.ppm`;if(!fs.existsSync(fn)){res.push('-');continue;}const b=readPpm(fn);let c=0;for(let i=0;i<a.rgb.length;i+=3)if(a.rgb[i]!==b.rgb[i]||a.rgb[i+1]!==b.rgb[i+1]||a.rgb[i+2]!==b.rgb[i+2])c++;res.push(c);}
 out.push(`${f} k=${k}: ${res.join(' ')}`);}
console.log(out.join('\n'));
