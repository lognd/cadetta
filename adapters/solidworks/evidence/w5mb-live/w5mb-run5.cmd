@echo off
set C=C:\src\nehy\cad\target\x86_64-pc-windows-msvc\release\cadetta.exe
set W=C:\src\w5mb\out\w5mb\W5mb.exe
set LOG=C:\src\w5mb\run5.log
cd /d C:\src\w5mb\proj
echo == import solidworks --update (dry) > %LOG%
%C% import solidworks parts/CoverPlate.SLDPRT --update >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == import solidworks --update --write >> %LOG%
%C% import solidworks parts/CoverPlate.SLDPRT --update --write >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == sync (no-op) >> %LOG%
%C% sync >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == lib.cdt length >> %LOG%
findstr /n "130 120" src\lib.cdt >> %LOG%
%W% docs %LOG%
