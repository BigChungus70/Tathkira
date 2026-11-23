!include LogicLib.nsh
!include nsDialogs.nsh

Var LaunchOnStartupCheckbox
Var LaunchOnStartupState

Function LaunchOnStartupPage
  StrCpy $LaunchOnStartupState 0
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Tathkira"
  ${If} $0 != ""
    StrCpy $LaunchOnStartupState 1
  ${EndIf}
  ReadRegStr $0 HKLM "Software\Microsoft\Windows\CurrentVersion\Run" "Tathkira"
  ${If} $0 != ""
    StrCpy $LaunchOnStartupState 1
  ${EndIf}

  !insertmacro MUI_HEADER_TEXT "Startup options" "Choose whether Tathkira launches when you sign in."
  nsDialogs::Create 1018
  Pop $0
  ${If} $0 == error
    Abort
  ${EndIf}

  ${NSD_CreateCheckbox} 0 20u 100% 12u "Launch Tathkira when I sign in"
  Pop $LaunchOnStartupCheckbox
  ${If} $LaunchOnStartupState == 1
    ${NSD_Check} $LaunchOnStartupCheckbox
  ${EndIf}
  nsDialogs::Show
FunctionEnd

Function LaunchOnStartupPageLeave
  ${NSD_GetState} $LaunchOnStartupCheckbox $LaunchOnStartupState
FunctionEnd

Page custom LaunchOnStartupPage LaunchOnStartupPageLeave

!macro NSIS_HOOK_POSTINSTALL
  ${If} $LaunchOnStartupState == ""
    StrCpy $LaunchOnStartupState 0
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Tathkira"
    ${If} $0 != ""
      StrCpy $LaunchOnStartupState 1
    ${EndIf}
  ${EndIf}

  CreateDirectory "$INSTDIR\config"
  FileOpen $0 "$INSTDIR\config\autostart.flag" w
  ${If} $LaunchOnStartupState == 1
    FileWrite $0 "true"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Tathkira" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
  ${Else}
    FileWrite $0 "false"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Tathkira"
  ${EndIf}
  FileClose $0

  DeleteRegValue HKLM "Software\Microsoft\Windows\CurrentVersion\Run" "Tathkira"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Tathkira"
  DeleteRegValue HKLM "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Tathkira"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Tathkira"
    DeleteRegValue HKLM "Software\Microsoft\Windows\CurrentVersion\Run" "Tathkira"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Tathkira"
    DeleteRegValue HKLM "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Tathkira"

    ${If} $DeleteAppDataCheckboxState = 1
      RMDir /r "$INSTDIR\config"
      RMDir /r "$APPDATA\com.tathkira.desktop"
      RMDir /r "$APPDATA\com.tathkira.app"
      RMDir /r "$LOCALAPPDATA\com.tathkira.desktop"
      RMDir /r "$LOCALAPPDATA\com.tathkira.app"
    ${EndIf}

    RMDir "$INSTDIR"
  ${EndIf}
!macroend
