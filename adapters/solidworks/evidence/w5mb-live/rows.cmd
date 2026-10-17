@echo off
set N=C:\src\nehy\ctlbin\NehyCtl.exe
for /L %%i in (0,1,9) do (
  %N% selectrow - %%i
  %N% call - MenuWhy
  ping -n 3 127.0.0.1 >nul
)
