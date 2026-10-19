; Runs after Tauri's own uninstall steps, which already remove the start-with-Windows (Run) entry.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ; Windows' on/off switch for that entry (Task Manager > Startup apps).
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "${PRODUCTNAME}"
    ; Settings live in %APPDATA%\Traywise, not the bundle-id folder Tauri's "Delete the application data" box clears.
    ${If} $DeleteAppDataCheckboxState = 1
      SetShellVarContext current
      RmDir /r "$APPDATA\Traywise"
      ; If Traywise was running, its WebView2 helpers outlive it by a moment and lock this folder.
      ${If} ${FileExists} "$LOCALAPPDATA\${BUNDLEID}\*.*"
        Sleep 3000
        RmDir /r "$LOCALAPPDATA\${BUNDLEID}"
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend
