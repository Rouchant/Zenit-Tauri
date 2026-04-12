!include "nsDialogs.nsh"
!include "LogicLib.nsh"

Var UninstDialog
Var UninstLabel
Var UninstPassword

!macro NSIS_HOOK_PREUNINSTALL
    nsDialogs::Create 1018
    Pop $UninstDialog

    ${If} $UninstDialog == error
        Abort
    ${EndIf}

    ${NSD_CreateLabel} 0 0 100% 24u "Acceso Restringido: Ingrese la contraseña de administrador para desinstalar la aplicación."
    Pop $UninstLabel

    ${NSD_CreatePassword} 0 30u 100% 12u ""
    Pop $UninstPassword

    nsDialogs::Show
    
    ${NSD_GetText} $UninstPassword $0
    ${If} $0 != "rogally"
        MessageBox MB_OK|MB_ICONSTOP "Contraseña incorrecta. Desinstalación cancelada."
        Quit
    ${EndIf}
!macroend
