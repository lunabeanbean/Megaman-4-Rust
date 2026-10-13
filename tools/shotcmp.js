// node tools/shotcmp.js harnessdir name rsdir [from to]: compares the Mesen screenshots of gen_ptrace.js SHOTS (shot_<name>_<frame>.png, shmap_<name>.txt = "frame lastrow")
// with the Rust pictures rs_<row>.ppm (MM4_SHOTDIR, MM4_SHOTS) at rows R+d for d = -1..2 and prints, per screenshot, the differing pixel counts (best d marked)
const fs=require('fs');const {readPng}=require('./pngcmp.js');
function readPpm(file){const b=fs.readFileSync(file);let p=0;const tok=()=>{while(b[p]===10||b[p]===32||b[p]===13)p++;let s='';while(b[p]!==10&&b[p]!==32&&b[p]!==13)s+=String.fromCharCode(b[p++]);return s;};tok();const w=+tok(),h=+tok();tok();p++;return {w,h,rgb:b.slice(p,p+w*h*3)};}
const [dir,name,rs]=process.argv.slice(2,5);const from=+(process.argv[5]||0),to=+(process.argv[6]||1e9);
const map=fs.readFileSync(`${dir}/shmap_${name}.txt`,'utf8').trim().split('\n').map(l=>l.split(' ').map(Number));
const hist={};let bad=0,n=0;
for(const [frame,row] of map){
  if(row<from||row>to)continue;
  let a;try{a=readPng(`${dir}/shot_${name}_${frame}.png`);}catch(e){continue;}
  const res=[];
  for(let d=-1;d<=2;d++){const f=`${rs}/rs_${row+d}.ppm`;if(!fs.existsSync(f)){res.push(null);continue;}const b=readPpm(f);let c=0;for(let i=0;i<a.rgb.length;i+=3)if(a.rgb[i]!==b.rgb[i]||a.rgb[i+1]!==b.rgb[i+1]||a.rgb[i+2]!==b.rgb[i+2])c++;res.push(c);}
  const best=res.reduce((bi,v,i)=>v!==null&&(bi<0||v<res[bi])?i:bi,-1);
  n++;hist[best-1]=(hist[best-1]||0)+1;if(res[best]>0)bad++;
  if(process.env.V)console.log(frame,row,res.join(' '),'best d',best-1);
}
console.log(`${n} screenshots, best row offset histogram ${JSON.stringify(hist)}, ${bad} not identical even at the best offset`);
