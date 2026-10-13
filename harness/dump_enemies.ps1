# List the enemy placement table of a stage. usage: dump_enemies.ps1 -Stage 2
param([int]$Stage = 2, [string]$Rom = "C:\Users\joele\Desktop\MegaMan4 Project\megaman4-disassembly\mm4.nes")
$bytes = [IO.File]::ReadAllBytes($Rom)
$base = 16 + ((0x20 -bor $Stage) * 8192)
function B($cpu) { $bytes[$base + $cpu - 0xA000] }
"idx  page:x   y   type"
for ($i = 0; $i -lt 128; $i++) {
  $pg = B (0xB100 + $i); if ($pg -eq 0xFF) { "end marker at index $i"; break }
  "{0,3}  {1:X2}:{2:X2}  {3:X2}  {4:X2}{5}" -f $i, $pg, (B (0xB180 + $i)), (B (0xB200 + $i)), (B (0xB280 + $i)), $(if ((B (0xB280 + $i)) -ge 0xC0) { "  (command)" } else { "" })
}
