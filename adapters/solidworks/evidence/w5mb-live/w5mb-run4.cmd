@echo off
set C=C:\src\nehy\cad\target\x86_64-pc-windows-msvc\release\cadetta.exe
set W=C:\src\w5mb\out\w5mb\W5mb.exe
set P=C:\src\w5mb\proj\parts\CoverPlate.SLDPRT
set LOG=C:\src\w5mb\run4.log
cd /d C:\src\w5mb\proj
echo == edit D1@Sketch2 to 130 in CAD > %LOG%
%W% edit %P% %LOG% D1@Sketch2 130
echo == sync (expect R001 drift) >> %LOG%
%C% sync >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == import --update >> %LOG%
%C% import --update >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == sync (no-op) >> %LOG%
%C% sync >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == names --observe (explicit only) >> %LOG%
%C% names --observe 2>&1 | findstr explicit >> %LOG%
%W% docs %LOG%
