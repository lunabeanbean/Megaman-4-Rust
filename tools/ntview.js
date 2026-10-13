// node tools/ntview.js dumpdir paletteHex out.png [bgbase=0]: draws the four nametables of chain_rec.lua's CH_CHRDUMP (nt.bin, chr.bin) as a 2x2 picture
const fs=require('fs');const {NES_RGB,writePng,decodeTile}=require('./nes.js');
const [dir,palhex,out]=process.argv.slice(2);const base=+(process.argv[5]||0);
const nt=fs.readFileSync(dir+'/nt.bin'),chr=fs.readFileSync(dir+'/chr.bin');const pal=Buffer.from(palhex,'hex');
const W=512,H=480,rgb=Buffer.alloc(W*H*3);
for(let n=0;n<4;n++)for(let ty=0;ty<30;ty++)for(let tx=0;tx<32;tx++){
 const o=n*0x400,tile=nt[o+ty*32+tx],attr=nt[o+0x3C0+(ty>>2)*8+(tx>>2)],p=(attr>>(((ty>>1)&1)*4+((tx>>1)&1)*2))&3;
 const px=decodeTile(chr,base+tile*16);
 for(let y=0;y<8;y++)for(let x=0;x<8;x++){const c=px[y*8+x];const col=NES_RGB[pal[c?p*4+c:0]&0x3F];const X=(n&1)*256+tx*8+x,Y=(n>>1)*240+ty*8+y;const i=(Y*W+X)*3;rgb[i]=col[0];rgb[i+1]=col[1];rgb[i+2]=col[2];}}
writePng(out,W,H,rgb);
