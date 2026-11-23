Unicode true
!include "MUI2.nsh"

Name "Oryn"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Oryn"
InstallDirRegKey HKCU "Software\Oryn" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\Oryn.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Oryn"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "Oryn installer"
VIAddVersionKey "LegalCopyright" "AGPL-3.0-only"

Section "Oryn"
  SetOutPath "$INSTDIR"
  File "/oname=Oryn.exe" "${EXE}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Oryn" "InstallDir" "$INSTDIR"
  CreateShortCut "$SMPROGRAMS\Oryn.lnk" "$INSTDIR\Oryn.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oryn" "DisplayName" "Oryn"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oryn" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oryn" "Publisher" "Kaventro"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oryn" "DisplayIcon" "$INSTDIR\Oryn.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oryn" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oryn" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oryn" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\Oryn.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Oryn.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oryn"
  DeleteRegKey HKCU "Software\Oryn"
SectionEnd
