#!/usr/bin/env node
// Mega Man 4 stage data tool: export one stage's tile/screen/enemy data to JSON and import it back into a ROM.
// Format documented in notes/level-format.md and notes/enemy-placement.md.
//
//   node tools/stage.js export <rom.nes> <stage 0-14> <out.json>
//   node tools/stage.js import <rom.nes> <stage.json> <out-rom.nes>
//   node tools/stage.js roundtrip <rom.nes> <stage>      (export, import, compare; must be byte-identical)
//
// The stage bank is 8 KB at ROM offset 0x10 + (0x20|stage)*0x2000, mapped at CPU $A000. Only the regions listed
// below are edited; every other byte of the bank is preserved by import.
'use strict';
const fs = require('fs');

const BANK = 0x2000, HEADER = 0x10;
const bankOffset = stage => HEADER + (0x20 | stage) * BANK;
const A = addr => addr - 0xA000;           // CPU address -> offset inside the bank

const hex = b => b.toString(16).toUpperCase().padStart(2, '0');

function exportStage(rom, stage) {
  const base = bankOffset(stage);
  const bank = rom.subarray(base, base + BANK);
  const metatiles = [];
  for (let m = 0; m < 256; m++) {
    const flags = bank[A(0xA400) + m];
    metatiles.push({
      tiles: [bank[A(0xA000) + m], bank[A(0xA100) + m], bank[A(0xA200) + m], bank[A(0xA300) + m]], // TL TR BL BR (8x8 tile ids)
      palette: flags & 3,
      type: flags >> 4,
      flagsRaw: flags,  // bits 2-3 are not understood; kept so import is lossless
    });
  }
  const blocks = [];
  for (let b = 0; b < 256; b++) blocks.push(Array.from(bank.subarray(A(0xA500) + b * 4, A(0xA500) + b * 4 + 4))); // TL TR BL BR metatile ids
  const layouts = [];
  for (let l = 0; l < 32; l++) layouts.push(Array.from(bank.subarray(A(0xA900) + l * 64, A(0xA900) + l * 64 + 64))); // 8x8 grid of block ids, row-major
  const screenOrder = Array.from(bank.subarray(A(0xB500), A(0xB500) + 0x30));  // layout id for each screen index
  const enemies = [];
  for (let i = 0; i < 128; i++) {
    const page = bank[A(0xB100) + i];
    if (page === 0xFF) break;
    enemies.push({ page, x: bank[A(0xB180) + i], y: bank[A(0xB200) + i], type: bank[A(0xB280) + i] });
  }
  return {
    format: 'mm4-stage-1', stage, bank: '0x' + hex(0x20 | stage),
    note: 'tiles = 8x8 CHR tile ids; blocks = 2x2 metatiles; layouts = 8x8 blocks; screenOrder[i] = layout of screen i; enemy type >= 0xC0 is a command, not an enemy',
    metatiles, blocks, layouts, screenOrder, enemies,
  };
}

function importStage(rom, stage, data) {
  if (data.format !== 'mm4-stage-1') throw new Error('unknown format ' + data.format);
  const out = Buffer.from(rom);
  const base = bankOffset(stage);
  const put = (cpu, i, v) => {
    if (!Number.isInteger(v) || v < 0 || v > 255) throw new Error(`byte out of range at ${cpu.toString(16)}+${i}: ${v}`);
    out[base + A(cpu) + i] = v;
  };
  if (data.metatiles.length !== 256) throw new Error('need 256 metatiles');
  data.metatiles.forEach((m, i) => {
    m.tiles.forEach((t, k) => put(0xA000 + 0x100 * k, i, t));
    // keep unknown flag bits (2-3) from flagsRaw unless the editor changed palette/type
    put(0xA400, i, (m.type << 4) | (m.flagsRaw & 0x0C) | m.palette);
  });
  if (data.blocks.length !== 256) throw new Error('need 256 blocks');
  data.blocks.forEach((b, i) => b.forEach((v, k) => put(0xA500, i * 4 + k, v)));
  if (data.layouts.length !== 32) throw new Error('need 32 layouts');
  data.layouts.forEach((l, i) => { if (l.length !== 64) throw new Error('layout needs 64 cells'); l.forEach((v, k) => put(0xA900, i * 64 + k, v)); });
  data.screenOrder.forEach((v, i) => put(0xB500, i, v));
  if (data.enemies.length > 127) throw new Error('at most 127 enemies');
  for (let i = 1; i < data.enemies.length; i++) {
    const p = data.enemies[i - 1], q = data.enemies[i];
    if (p.page > q.page) { console.warn(`warning: stage ${stage} enemy ${i} is not sorted by page (the spawner expects entries in screen order)`); break; }
  }
  data.enemies.forEach((e, i) => { put(0xB100, i, e.page); put(0xB180, i, e.x); put(0xB200, i, e.y); put(0xB280, i, e.type); });
  put(0xB100, data.enemies.length, 0xFF);   // end marker (leftover entries after it are ignored by the game)
  return out;
}

function main() {
  const [cmd, ...a] = process.argv.slice(2);
  if (cmd === 'export') {
    const rom = fs.readFileSync(a[0]);
    fs.writeFileSync(a[2], JSON.stringify(exportStage(rom, +a[1]), null, 1));
    console.log(`exported stage ${a[1]} -> ${a[2]}`);
  } else if (cmd === 'import') {
    const rom = fs.readFileSync(a[0]);
    const data = JSON.parse(fs.readFileSync(a[1], 'utf8'));
    fs.writeFileSync(a[2], importStage(rom, data.stage, data));
    console.log(`imported stage ${data.stage} -> ${a[2]}`);
  } else if (cmd === 'roundtrip') {
    const rom = fs.readFileSync(a[0]);
    const out = importStage(rom, +a[1], JSON.parse(JSON.stringify(exportStage(rom, +a[1]))));
    const same = Buffer.compare(rom, out) === 0;
    // the end marker rewrite can only change a byte if the original had no marker; report any difference
    let diffs = 0; for (let i = 0; i < rom.length; i++) if (rom[i] !== out[i]) diffs++;
    console.log(same ? `stage ${a[1]}: round trip OK (ROM byte-identical)` : `stage ${a[1]}: ${diffs} byte(s) differ`);
    process.exit(same ? 0 : 1);
  } else {
    console.log('usage: node tools/stage.js export|import|roundtrip ...  (see header of the file)');
    process.exit(2);
  }
}
main();
