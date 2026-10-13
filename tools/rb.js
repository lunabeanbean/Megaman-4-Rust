const fs=require('fs');const rom=fs.readFileSync('Mega Man 4 (USA).nes');
const [,, bank, a, n]=process.argv;const off=16+parseInt(bank,16)*0x2000+(parseInt(a,16)-0x8000);
let s='';for(let i=0;i<parseInt(n);i++){s+=rom[off+i].toString(16).padStart(2,'0')+' ';if(i%16==15)s+='\n'}console.log(s);
