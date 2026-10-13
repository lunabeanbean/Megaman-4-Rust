param($script, $rom = "C:\Users\joele\Desktop\MegaMan4 Project\megaman4-disassembly\mm4.nes")
# Several of these can run at the same time (game_test.sh JOBS=n): only the Mesen this call started is ever stopped.
$a = "--testRunner `"$rom`" `"C:\Users\joele\Desktop\MegaMan4 Project\harness\$script`""
$p = Start-Process "C:\Users\joele\Downloads\Mesen\Mesen.exe" -ArgumentList $a -PassThru
$limit = 240; if ($env:RT_TIMEOUT) { $limit = [int]$env:RT_TIMEOUT }
if (-not $p.WaitForExit($limit * 1000)) { $p.Kill(); "timeout" }
"exit $($p.ExitCode)"
