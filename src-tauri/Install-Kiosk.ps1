param (
    [Parameter(Mandatory=$false)]
    [string]$InstallDir = "$env:LOCALAPPDATA\zenit"
)

# 1. Ajustes de Energía (PowerCfg)
$guids = powercfg /l | Select-String -Pattern '([a-fA-F0-9]{8}-([fA-F0-9]{4}-){3}[a-fA-F0-9]{12})' | ForEach-Object { $_.Matches.Value }
if ($null -eq $guids) { $guids = @("SCHEME_CURRENT") }

foreach ($guid in $guids) {
    powercfg /setacvalueindex $guid SUB_SLEEP HIBERNATEIDLE 0
    powercfg /setacvalueindex $guid SUB_SLEEP STANDBYIDLE 0
    powercfg /setacvalueindex $guid SUB_VIDEO VIDEOIDLE 0
}

powercfg /s SCHEME_CURRENT
powercfg /hibernate off
powercfg /setacvalueindex SCHEME_CURRENT SUB_VIDEO ADAPTBRIGHT 0
powercfg /setdcvalueindex SCHEME_CURRENT SUB_VIDEO ADAPTBRIGHT 0
powercfg /setacvalueindex SCHEME_CURRENT SUB_VIDEO VIDEOQUALITY 1
powercfg /setdcvalueindex SCHEME_CURRENT SUB_VIDEO VIDEOQUALITY 1

# 2. Hardening y Registro
$paths = @(
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\PushNotifications",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth\QuickPair",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth"
)
foreach ($p in $paths) { if (!(Test-Path $p)) { New-Item -Path $p -Force | Out-Null } }

Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings' -Name 'NOC_GLOBAL_SETTING_TOASTS_ENABLED' -Value 0 -ErrorAction SilentlyContinue
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings' -Name 'FocusAssistState' -Value 2 -ErrorAction SilentlyContinue
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\PushNotifications' -Name 'ToastEnabled' -Value 0 -ErrorAction SilentlyContinue

Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth\QuickPair' -Name 'QuickPairEnabled' -Value 0 -ErrorAction SilentlyContinue
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Bluetooth' -Name 'SwiftPairDefault' -Value 0 -ErrorAction SilentlyContinue

$tpPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\PrecisionTouchPad'
if (!(Test-Path $tpPath)) { New-Item -Path $tpPath -Force | Out-Null }
Set-ItemProperty -Path $tpPath -Name 'ThreeFingerAndFourFingerGestures' -Value 0 -ErrorAction SilentlyContinue

$gestures = @('ThreeFingerSwipeUp', 'ThreeFingerSwipeDown', 'ThreeFingerSwipeLeft', 'ThreeFingerSwipeRight',
              'FourFingerSwipeUp', 'FourFingerSwipeDown', 'FourFingerSwipeLeft', 'FourFingerSwipeRight',
              'ThreeFingerTap', 'FourFingerTap')
foreach ($g in $gestures) { Set-ItemProperty -Path $tpPath -Name $g -Value 0 -ErrorAction SilentlyContinue }

Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowTaskViewButton' -Value 0 -ErrorAction SilentlyContinue
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'EnableEdgeSwipe' -Value 0 -ErrorAction SilentlyContinue

$explorerPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer'
Set-ItemProperty -Path "$explorerPath\Advanced" -Name 'VirtualDesktopTaskbarFilter' -Value 1 -ErrorAction SilentlyContinue
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager' -Name 'SubscribedContent-338388Enabled' -Value 0 -ErrorAction SilentlyContinue

$policyPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer'
if (!(Test-Path $policyPath)) { New-Item -Path $policyPath -Force | Out-Null }
Set-ItemProperty -Path $policyPath -Name 'NoWindowMinimizingShortcuts' -Value 1 -ErrorAction SilentlyContinue

Stop-Service -Name "vdmss" -Force -ErrorAction SilentlyContinue
Set-Service -Name "vdmss" -StartupType Disabled -ErrorAction SilentlyContinue

# 3. Exclusiones de Antivirus (Windows Defender)
Add-MpPreference -ExclusionPath $InstallDir -ErrorAction SilentlyContinue
Add-MpPreference -ExclusionProcess "zenit.exe" -ErrorAction SilentlyContinue
