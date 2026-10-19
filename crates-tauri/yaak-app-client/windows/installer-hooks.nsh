!macro NSIS_HOOK_PREINSTALL
  ; Stop the plugin runtime if Yaak exited without cleaning it up. Otherwise it
  ; can keep yaaknode.exe locked while the installer tries to replace it.
  !insertmacro CheckIfAppIsRunning "yaaknode.exe" "Yaak"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CheckIfAppIsRunning "yaaknode.exe" "Yaak"
!macroend
