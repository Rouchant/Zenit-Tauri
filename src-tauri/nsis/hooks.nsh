!include "nsDialogs.nsh"
!include "LogicLib.nsh"

; El desinstalador ahora es estándar y no requiere contraseña.
!macro NSIS_HOOK_PREUNINSTALL
    ; Sin restricciones.
!macroend
