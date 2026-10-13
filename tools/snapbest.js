// node tools/snapbest.js harnessdir name rsdir shotrow snaprow [step]: for every shot of the row the best-matching per-iteration picture (all k) and its difference
const fs=require('fs');const {readPng}=require('./pngcmp.js');
function readPpm(file){const b=fs.readFileSync(file);let p=0;const tok=()=>{while(b[p]===10||b[p]===32||b[p]===13)p++;let s='';while(b[p]!==10&&b[p]!==32&&b[p]!==13)s+=String.fromCharCode(b[p++]);return s;};tok();const w=+tok(),h=+tok();tok();p++;return {w,h,rgb:b.slice(p,p+w*h*3)};}
const [dir,name,rs,srow,prow]=process.argv.slice(2,7);const step=+(process.argv[7]||1);
const map=fs.readFileSync(`${dir}/shmap_${name}.txt`,'utf8').trim().split('\n').map(l=>l.split(' ').map(Number)).filter(x=>x[1]===+srow);
const snaps=[];for(let k=0;fs.existsSync(`${rs}/rs_${prow}_${k}.ppm`);k++)snaps.push(readPpm(`${rs}/rs_${prow}_${k}.ppm`));
const f0=map[0][0];let n=0;
for(const [f] of map){if((f-f0)%step)continue;const a=readPng(`${dir}/shot_${name}_${f}.png`);let best=[1e9,-1];
 snaps.forEach((b,k)=>{let c=0;for(let i=0;i<a.rgb.length;i+=3)if(a.rgb[i]!==b.rgb[i]||a.rgb[i+1]!==b.rgb[i+1]||a.rgb[i+2]!==b.rgb[i+2]){c++;if(c>=best[0])return;}if(c<best[0])best=[c,k];});
 console.log(`${f} (k=${f-f0}): best snap ${best[1]} (offset ${best[1]-(f-f0)}) diff ${best[0]}`);}
