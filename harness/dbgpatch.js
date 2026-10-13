// node dbgpatch.js <seed> <lo> <hi> <addr> [write|exec]  -> dbg_<seed>.lua logging writes/exec at addr for logic frames lo..hi into dbg_pc.txt
const fs=require("fs");
const [seed,lo,hi,addr,kind]=process.argv.slice(2);
let s=fs.readFileSync(`ptrace_r${seed}.lua`,"utf8");
for(const k of ["ptrace","otrace","xtrace","atrace"]) s=s.split(`${k}_r${seed}.txt`).join(`dbg_${k}.txt`);
const ex = kind==="exec";
const hook=`local dbg = io.open("C:/Users/joele/Desktop/MegaMan4 Project/harness/dbg_pc.txt","w")
emu.addMemoryCallback(function(a,v) if nlog>=${lo} and nlog<=${hi} then local st=emu.getState(); dbg:write(nlog," ",string.format("%04X",a)," v=",string.format("%02X",v or 0)," pc=",string.format("%04X",st["cpu.pc"])," x=",string.format("%02X",st["cpu.x"])," y=",string.format("%02X",st["cpu.y"]),string.char(10)); dbg:flush() end end, emu.callbackType.${ex?"exec":"write"}, ${addr})
`;
s=s.replace("local nlog = -1\n","local nlog = -1\n"+hook);
fs.writeFileSync(`dbg_${seed}.lua`,s);
