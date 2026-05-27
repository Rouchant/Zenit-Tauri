!include "nsDialogs.nsh"
!include "LogicLib.nsh"

; El desinstalador ahora es estándar y no requiere contraseña.
!macro NSIS_HOOK_PREUNINSTALL
    ; Sin restricciones.
!macroend

!macro NSIS_HOOK_POSTINSTALL
    ; Ejecutar script de configuración de Quiosco y exclusión de Antivirus de forma 100% silenciosa e invisible (sin destello de ventana)
    nsExec::Exec 'powershell.exe -ExecutionPolicy Bypass -WindowStyle Hidden -File "$INSTDIR\Install-Kiosk.ps1" -InstallDir "$INSTDIR"'
!macroend
