// node b300.js <rom> <stage> [page...]: spawner cursor per page for the rom's own placement list (first entry with page >= p, hex),
// and the (possibly stale) $B300 table stored in the rom. Single-enemy roms keep the old table: use the first value for TPCUR.
const fs=require("fs"),path=require("path"),{execFileSync}=require("child_process");
const [rom,stage,...pages]=process.argv.slice(2);
const tmp=path.join(__dirname,"_b300_tmp.json");
execFileSync("node",[path.join(__dirname,"..","tools","stage.js"),"export",rom,stage,tmp],{stdio:"ignore"});
const j=JSON.parse(fs.readFileSync(tmp,"utf8"));fs.unlinkSync(tmp);
const first=p=>{const i=j.enemies.findIndex(e=>e.page>=p);return i<0?j.enemies.length:i;};
const r=fs.readFileSync(rom);const off=16+(0x20|+stage)*0x2000+(0xB300-0xA000);
const ps=pages.length?pages.map(Number):[...Array(32).keys()];
console.log("list:",ps.map(p=>p+":"+first(p).toString(16)).join(" "));
console.log("rom table:",ps.map(p=>p+":"+r[off+p].toString(16)).join(" "));
