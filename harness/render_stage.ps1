# Debug render of a stage's collision-type map from the ROM. usage: render_stage.ps1 -Stage 2 -Screens 23
param([int]$Stage = 2, [int]$Screens = 23, [string]$Rom = "C:\Users\joele\Desktop\MegaMan4 Project\megaman4-disassembly\mm4.nes",
      [string]$Out = "C:\Users\joele\Desktop\MegaMan4 Project\harness\stage_types.png")
Add-Type -AssemblyName System.Drawing
$bytes = [IO.File]::ReadAllBytes($Rom)
$base = 16 + ((0x20 -bor $Stage) * 8192)          # stage data bank is $20|stage, mapped at $A000
function B($cpu) { $bytes[$base + $cpu - 0xA000] }
$cell = 8   # pixels per 16x16 metatile in the picture
$w = $Screens * 16 * $cell; $h = 16 * $cell
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp); $g.Clear([System.Drawing.Color]::Black)
$cols = 0..15 | % { $v = ($_ * 16 + 40) % 256; [System.Drawing.Color]::FromArgb(255, ($_ * 53) % 200 + 55, ($_ * 97) % 200 + 55, ($_ * 29) % 200 + 55) }
$cols[0] = [System.Drawing.Color]::FromArgb(255, 20, 20, 40)   # type 0 = empty
for ($s = 0; $s -lt $Screens; $s++) {
  $layout = B (0xB500 + $s)
  for ($cellIdx = 0; $cellIdx -lt 64; $cellIdx++) {
    $blockCol = $cellIdx % 8; $blockRow = [math]::Floor($cellIdx / 8)
    $blk = B (0xA900 + $layout * 64 + $cellIdx)
    for ($m = 0; $m -lt 4; $m++) {
      $meta = B (0xA500 + $blk * 4 + $m)
      $type = (B (0xA400 + $meta)) -shr 4
      $mx = $blockCol * 2 + ($m % 2); $my = $blockRow * 2 + [math]::Floor($m / 2)
      if ($my -ge 15) { continue }
      $brush = New-Object System.Drawing.SolidBrush $cols[$type]
      $g.FillRectangle($brush, ($s * 16 + $mx) * $cell, $my * $cell, $cell - 1, $cell - 1); $brush.Dispose()
    }
  }
}
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose(); "saved $Out ($w x $h)"
