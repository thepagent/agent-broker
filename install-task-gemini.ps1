$user = "$env:COMPUTERNAME\Administrator"
Unregister-ScheduledTask -TaskName "OpenAB-Gemini" -Confirm:$false -ErrorAction SilentlyContinue

$action = New-ScheduledTaskAction -Execute "C:\Users\Administrator\openab\run-openab-gemini.bat"
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $user
$settings = New-ScheduledTaskSettingsSet `
  -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable `
  -ExecutionTimeLimit (New-TimeSpan -Seconds 0) `
  -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1)
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest
Register-ScheduledTask -TaskName "OpenAB-Gemini" `
  -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force

Start-ScheduledTask -TaskName "OpenAB-Gemini"
Start-Sleep 3
Get-ScheduledTask -TaskName "OpenAB-Gemini" | Format-List TaskName,State
