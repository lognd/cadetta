@echo off
set C=C:\src\nehy\cad\target\x86_64-pc-windows-msvc\release\cadetta.exe
set W=C:\src\s1\out\w5mb\W5mb.exe
set LOG=C:\src\s1\run.log
set P=C:\src\s1\proj\parts\CoverPlate.SLDPRT
cd /d C:\src\s1\proj
echo == docs before > %LOG%
%W% docs %LOG% docs
echo == build part (120 x 80 mm plate, linear pattern count 4) >> %LOG%
%W% build %P% %LOG%
echo == link pattern count D1@LPattern1 to global bolt_count (LinkCommands, fixed build) >> %LOG%
%W% link %P% %LOG% D1@LPattern1 bolt_count
echo == link D1@Sketch3 to global pilot_bore >> %LOG%
%W% link %P% %LOG% D1@Sketch3 pilot_bore
echo == names --observe >> %LOG%
%C% names --observe >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == sync (baseline) >> %LOG%
%C% sync >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == sync again (no-op) >> %LOG%
%C% sync >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == edit D1@Sketch2 to 130 in CAD >> %LOG%
%W% edit %P% %LOG% D1@Sketch2 130
echo == sync (expect R001 drift) >> %LOG%
%C% sync >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == import solidworks --update (dry) >> %LOG%
%C% import solidworks parts/CoverPlate.SLDPRT --update >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == import solidworks --update --write >> %LOG%
%C% import solidworks parts/CoverPlate.SLDPRT --update --write >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == sync (no-op) >> %LOG%
%C% sync >> %LOG% 2>&1
echo exit=%ERRORLEVEL% >> %LOG%
echo == lib.cdt length and bolt_count >> %LOG%
findstr /n "length bolt_count" src\lib.cdt >> %LOG%
echo == docs after >> %LOG%
%W% docs %LOG% docs
echo done > C:\src\s1\run.done
