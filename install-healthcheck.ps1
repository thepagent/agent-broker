$user = "$env:COMPUTERNAME\Administrator"
Unregister-ScheduledTask -TaskName "OpenAB-Healthcheck" -Confirm:$false -ErrorAction SilentlyContinue

$action = New-ScheduledTaskAction `
    -Execute "powershell.exe" `
    -Argument "-ExecutionPolicy Bypass -WindowStyle Hidden -File `"C:\Users\Administrator\openab\openab-healthcheck.ps1`""

$trigger = New-ScheduledTaskTrigger -Once -At (Get-Date) `
    -RepetitionInterval (New-TimeSpan -Minutes 2) `
    -RepetitionDuration (New-TimeSpan -Days 9999)

$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable `
    -ExecutionTimeLimit (New-TimeSpan -Minutes 1)

$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest

Register-ScheduledTask -TaskName "OpenAB-Healthcheck" `
    -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force

Start-ScheduledTask -TaskName "OpenAB-Healthcheck"
Start-Sleep 3
Get-ScheduledTask -TaskName "OpenAB-Healthcheck" | Format-Table TaskName, State -AutoSize
