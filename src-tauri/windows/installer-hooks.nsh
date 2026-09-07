; Embed ONNX Runtime's DirectML dependency next to stepdoc.exe.
; Path is resolved at installer compile time (target/release/DirectML.dll).

!macro NSIS_HOOK_PREINSTALL
  File "/oname=DirectML.dll" "..\..\DirectML.dll"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\DirectML.dll"
!macroend
