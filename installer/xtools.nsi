; Tauri renders this NSIS template before calling makensis.
; Current-user installation, optional shortcuts/startup, no network bootstrapper.
Unicode true
!include "MUI2.nsh"
!include "nsDialogs.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "FileFunc.nsh"
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!include "Win\RestartManager.nsh"
!include "utils.nsh"
!define BUNDLEID "com.antique.xtools"
Name "{{product_name}}"
OutFile "{{out_file}}"
InstallDir "$LOCALAPPDATA\Programs\XTools"
InstallDirRegKey HKCU "Software\XTools" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ManifestDPIAware true
ManifestDPIAwareness PerMonitorV2
VIProductVersion "{{version_with_build}}"
VIAddVersionKey "ProductName" "XTools"
VIAddVersionKey "FileDescription" "XTools installer"
VIAddVersionKey "FileVersion" "{{version}}"
Var OptionsDialog
Var DesktopCheckbox
Var StartupCheckbox
Var DesktopChoice
Var StartupChoice
!define XTOOLS_EXE "{{main_binary_name}}.exe"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${XTOOLS_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "启动 XTools"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
Page custom OptionsPage OptionsLeave
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${IsNativeAMD64}
    MessageBox MB_ICONSTOP "XTools V2 只支持 Windows 10（1703 及以上）/ Windows 11 x64。"
    Abort
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_ICONSTOP "XTools V2 需要 Windows 10（1703 及以上）或 Windows 11。"
    Abort
  ${EndIf}
  ; SetProcessDpiAwarenessContext / Per-Monitor V2 require Windows 10 1703.
  ; WinVer.nsh provides AtLeastBuild; 1703 corresponds to build 15063.
  ${IfNot} ${AtLeastBuild} 15063
    MessageBox MB_ICONSTOP "XTools V2 的多显示器 DPI 支持需要 Windows 10 1703（build 15063）或更高版本。请升级 Windows 后重试。"
    Abort
  ${EndIf}
  ; No network is used during setup. WebView2 is a system prerequisite.
  SetRegView 32
  ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${If} $0 == ""
    ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${EndIf}
  SetRegView 64
  ${If} $0 == ""
    MessageBox MB_ICONSTOP "此电脑尚未安装 Microsoft Edge WebView2 Runtime。请先安装官方 WebView2 离线运行时，然后重新运行 XTools 安装程序。安装器不会联网下载。"
    Abort
  ${EndIf}
  SetShellVarContext current
  StrCpy $DesktopChoice ${BST_CHECKED}
  StrCpy $StartupChoice ${BST_CHECKED}
  ${GetParameters} $0
  ClearErrors
  ${GetOptions} $0 "/NODESKTOP" $1
  ${IfNot} ${Errors}
    StrCpy $DesktopChoice ${BST_UNCHECKED}
  ${EndIf}
  ClearErrors
  ${GetOptions} $0 "/NOAUTOSTART" $1
  ${IfNot} ${Errors}
    StrCpy $StartupChoice ${BST_UNCHECKED}
  ${EndIf}
FunctionEnd

Function OptionsPage
  !insertmacro MUI_HEADER_TEXT "附加选项" "选择快捷方式和开机启动设置。"
  nsDialogs::Create 1018
  Pop $OptionsDialog
  ${If} $OptionsDialog == error
    Abort
  ${EndIf}
  ${NSD_CreateLabel} 0 0 100% 36u "XTools 安装后驻留托盘，按 Alt + Space 呼出。"
  Pop $0
  ${NSD_CreateCheckbox} 0 46u 100% 14u "创建桌面快捷方式"
  Pop $DesktopCheckbox
  ${NSD_SetState} $DesktopCheckbox $DesktopChoice
  ${NSD_CreateCheckbox} 0 70u 100% 14u "开机自动启动"
  Pop $StartupCheckbox
  ${NSD_SetState} $StartupCheckbox $StartupChoice
  nsDialogs::Show
FunctionEnd

Function OptionsLeave
  ${NSD_GetState} $DesktopCheckbox $DesktopChoice
  ${NSD_GetState} $StartupCheckbox $StartupChoice
FunctionEnd

Function un.onInit
  ; The uninstaller starts as a new 32-bit NSIS process. Match the
  ; installer's registry view before removing application-owned values.
  SetRegView 64
  SetShellVarContext current
FunctionEnd

Section "XTools" SEC_MAIN
  SetShellVarContext current
  ${If} ${FileExists} "$INSTDIR\${XTOOLS_EXE}"
    ExecWait '"$INSTDIR\${XTOOLS_EXE}" --quit'
    Sleep 500
  ${EndIf}
  SetOutPath "$INSTDIR"
  ClearErrors
  File "{{main_binary_path}}"
  ${If} ${Errors}
    MessageBox MB_ICONSTOP "无法写入程序文件。请关闭正在运行的 XTools，并确认目录可写，然后重试。"
    Abort
  ${EndIf}
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\XTools"
  CreateShortcut "$SMPROGRAMS\XTools\XTools.lnk" "$INSTDIR\${XTOOLS_EXE}"
  !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\XTools\XTools.lnk"
  ${If} $DesktopChoice == ${BST_CHECKED}
    CreateShortcut "$DESKTOP\XTools.lnk" "$INSTDIR\${XTOOLS_EXE}"
  ${Else}
    Delete "$DESKTOP\XTools.lnk"
  ${EndIf}
  WriteRegStr HKCU "Software\XTools" "InstallDir" "$INSTDIR"
  ${If} $StartupChoice == ${BST_CHECKED}
    WriteRegDWORD HKCU "Software\XTools" "AutostartPreference" 1
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "XTools" '$\"$INSTDIR\${XTOOLS_EXE}$\" --autostart'
  ${Else}
    WriteRegDWORD HKCU "Software\XTools" "AutostartPreference" 0
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "XTools"
  ${EndIf}
  ; The marker makes installer choices apply once even to an existing profile.
  WriteRegDWORD HKCU "Software\XTools" "ApplyInstallerPreference" 1
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools" "DisplayName" "XTools"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools" "DisplayVersion" "{{version}}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools" "Publisher" "Antique"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools" "DisplayIcon" "$INSTDIR\${XTOOLS_EXE}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  ExecWait '"$INSTDIR\${XTOOLS_EXE}" --quit'
  StrCpy $0 0
  uninstall_retry:
  ClearErrors
  Delete "$INSTDIR\${XTOOLS_EXE}"
  ${If} ${Errors}
    IntOp $0 $0 + 1
    ${If} $0 < 20
      Sleep 250
      Goto uninstall_retry
    ${EndIf}
    MessageBox MB_ICONSTOP "XTools 仍在运行或程序文件无法删除。请关闭 XTools 后重新卸载；安装信息已保留。"
    Abort
  ${EndIf}
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "XTools"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools"
  DeleteRegKey HKCU "Software\XTools"
  Delete "$DESKTOP\XTools.lnk"
  Delete "$SMPROGRAMS\XTools\XTools.lnk"
  RMDir "$SMPROGRAMS\XTools"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  ; %APPDATA%\XTools is preserved; never recursively delete user data.
SectionEnd
