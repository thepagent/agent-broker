[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$user = "$env:COMPUTERNAME\Administrator"
$pass = "MyNewPass123!"

# Kill everything
Stop-ScheduledTask OpenAB-Claude  -EA SilentlyContinue
Stop-ScheduledTask OpenAB-Copilot -EA SilentlyContinue
Stop-ScheduledTask OpenAB-Gemini  -EA SilentlyContinue
Stop-ScheduledTask OpenAB-Codex   -EA SilentlyContinue
Start-Sleep 1
Stop-Process -Name openab -Force -EA SilentlyContinue
Stop-Process -Name cmd -Force -EA SilentlyContinue
Start-Sleep 2

# Unregister all
@("OpenAB-Claude","OpenAB-Copilot","OpenAB-Gemini","OpenAB-Codex") | ForEach-Object {
    Unregister-ScheduledTask -TaskName $_ -Confirm:$false -EA SilentlyContinue
}

# Register all as AtLogOn + Interactive (original working config)
$bots = @(
    @{ Name="OpenAB-Claude";  Bat="run-openab-claude.bat"  },
    @{ Name="OpenAB-Copilot"; Bat="run-openab-copilot.bat" },
    @{ Name="OpenAB-Gemini";  Bat="run-openab-gemini.bat"  },
    @{ Name="OpenAB-Codex";   Bat="run-openab-codex.bat"   }
)

foreach ($bot in $bots) {
    # Use wscript + run-hidden.vbs to launch bat without visible window
    $action   = New-ScheduledTaskAction `
        -Execute "wscript.exe" `
        -Argument "`"C:\Users\Administrator\openab\run-hidden.vbs`" `"C:\Users\Administrator\openab\$($bot.Bat)`""
    $trigger  = New-ScheduledTaskTrigger -AtLogOn -User $user
    $settings = New-ScheduledTaskSettingsSet `
        -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable `
        -ExecutionTimeLimit (New-TimeSpan -Seconds 0) `
        -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1)
    $principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest

    Register-ScheduledTask -TaskName $bot.Name `
        -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force | Out-Null
    Write-Host "[OK] $($bot.Name) registered (headless via VBS)" -ForegroundColor Green
}

# Set Auto-Logon
Set-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon" -Name AutoAdminLogon -Value "1"
Set-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon" -Name DefaultUserName -Value "Administrator"
Set-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon" -Name DefaultPassword -Value $pass
Write-Host "[OK] Auto-Logon enabled" -ForegroundColor Green

# Start all tasks now
foreach ($bot in $bots) {
    Start-ScheduledTask -TaskName $bot.Name
}
Write-Host "[OK] All 4 tasks started" -ForegroundColor Cyan

Start-Sleep 8

# Verify
Write-Host ""
Write-Host "=== Task Status ===" -ForegroundColor Yellow
Get-ScheduledTask -TaskName "OpenAB-*" | Format-Table TaskName, State -AutoSize

Write-Host "=== Processes ===" -ForegroundColor Yellow
Get-Process -Name openab -EA SilentlyContinue | Format-Table Id, ProcessName, StartTime -AutoSize

Write-Host "=== Auto-Logon ===" -ForegroundColor Yellow
$al = (Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon").AutoAdminLogon
$du = (Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon").DefaultUserName
Write-Host "  AutoAdminLogon=$al  DefaultUserName=$du"
