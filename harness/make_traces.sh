#!/bin/sh
# Regenerates the player traces used by `mm4 --verify` (needs Mesen, see harness/README.md). Run from this folder.
# a, b use the unmodified ROM; c, d use harness/mm4_noenemy.nes (stage 2 with the enemy list removed, made with tools/stage.js)
NOENEMY='C:\Users\joele\Desktop\MegaMan4 Project\harness\mm4_noenemy.nes'
powershell -ExecutionPolicy Bypass -File rt.ps1 ptrace_a.lua
powershell -ExecutionPolicy Bypass -File rt.ps1 ptrace_b.lua
powershell -ExecutionPolicy Bypass -File rt.ps1 ptrace_c.lua "$NOENEMY"
powershell -ExecutionPolicy Bypass -File rt.ps1 ptrace_d.lua "$NOENEMY"
