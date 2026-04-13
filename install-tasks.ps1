# Remove old tasks if exist
Unregister-ScheduledTask -TaskName "OpenAB-Claude" -Confirm:$false -ErrorAction SilentlyContinue
Unregister-ScheduledTask -TaskName "OpenAB-Copilot" -Confirm:$false -ErrorAction SilentlyContinue

$user = "$env:COMPUTERNAME\Administrator"

# Claude task
$actionC = New-ScheduledTaskAction -Execute "C:\Users\Administrator\openab\run-openab-claude.bat"
$triggerC = New-ScheduledTaskTrigger -AtLogOn -User $user
$settingsC = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Seconds 0) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1)
$principalC = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest
Register-ScheduledTask -TaskName "OpenAB-Claude" -Action $actionC -Trigger $triggerC -Settings $settingsC -Principal $principalC -Force

# Copilot task
$actionP = New-ScheduledTaskAction -Execute "C:\Users\Administrator\openab\run-openab-copilot.bat"
$triggerP = New-ScheduledTaskTrigger -AtLogOn -User $user
$settingsP = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Seconds 0) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1)
$principalP = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest
Register-ScheduledTask -TaskName "OpenAB-Copilot" -Action $actionP -Trigger $triggerP -Settings $settingsP -Principal $principalP -Force

# Start them now
Start-ScheduledTask -TaskName "OpenAB-Claude"
Start-ScheduledTask -TaskName "OpenAB-Copilot"

Start-Sleep 2
Get-ScheduledTask -TaskName "OpenAB-*" | Get-ScheduledTaskInfo | Format-Table TaskName,LastRunTime,LastTaskResult
