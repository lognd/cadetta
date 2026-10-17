@echo off
set N=C:\src\nehy\ctlbin\NehyCtl.exe
%N% selectdim - D1@Sketch3
%N% selectrow - 7
%N% call - MenuLink
ping -n 4 127.0.0.1 >nul
%N% selectdim - D1@LPattern1
%N% selectrow - 0
%N% call - MenuLink
ping -n 4 127.0.0.1 >nul
