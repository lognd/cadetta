# Runs the SOLIDWORKS API probe in the interactive console session (session 1)
# of a Windows host, from a non-interactive shell such as ssh. Non-interactive.
#
#   powershell -File tools\probe-api.ps1 [-Exe <path>] [-Scratch <dir>] [-TimeoutSecs 300]
#
# SOLIDWORKS must already be open in the console session: attaching from
# session 0 fails (running object table is per logon session). Output goes to
# <Scratch>\probe-result.md and <Scratch>\probe.err; the window exits by itself.
param(
    [string]$Exe = (Join-Path $PSScriptRoot "..\src\cadetta-solidworks\bin\Release\net48\cadetta-solidworks.exe"),
    [string]$Scratch = "C:\src\probe",
    [int]$TimeoutSecs = 300
)
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force $Scratch | Out-Null
$cmd = "set CADETTA_LOG=debug&& `"$Exe`" --probe-api `"$Scratch`" > `"$Scratch\probe.out`" 2> `"$Scratch\probe.err`""
$user = "$env:COMPUTERNAME\$env:USERNAME"
$action = New-ScheduledTaskAction -Execute "cmd.exe" -Argument "/c $cmd"
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
    -ExecutionTimeLimit (New-TimeSpan -Seconds $TimeoutSecs)
Register-ScheduledTask -TaskName CadettaProbe -Action $action -Principal $principal -Settings $settings -Force | Out-Null
Start-ScheduledTask CadettaProbe
$end = (Get-Date).AddSeconds($TimeoutSecs)
do { Start-Sleep 2 } while ((Get-ScheduledTask CadettaProbe).State -eq "Running" -and (Get-Date) -lt $end)
"exit: " + (Get-ScheduledTaskInfo CadettaProbe).LastTaskResult
Unregister-ScheduledTask CadettaProbe -Confirm:$false
Get-Content "$Scratch\probe-result.md" -ErrorAction SilentlyContinue
