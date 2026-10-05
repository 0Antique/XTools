# Optional helper for this workspace's portable tools. Standard installed
# Rust/Visual Studio/Node environments do not need this script.
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskPortable = Join-Path $taskRoot '.tools\msvc'
if (Test-Path -LiteralPath $taskPortable) {
    $taskVc = (Get-ChildItem -LiteralPath (Join-Path $taskPortable 'VC\Tools\MSVC') -Directory | Sort-Object Name | Select-Object -Last 1).FullName
    $taskSdk = Join-Path $taskPortable 'Windows Kits\10'
    $taskSdkVersion = (Get-ChildItem -LiteralPath (Join-Path $taskSdk 'Lib') -Directory | Sort-Object Name | Select-Object -Last 1).Name
    $taskCompiler = Join-Path $taskVc 'bin\Hostx64\x64'
    $env:Path = "$taskCompiler;$taskSdk\bin\$taskSdkVersion\x64;" + $env:Path
    $env:INCLUDE = "$taskVc\include;$taskSdk\Include\$taskSdkVersion\ucrt;$taskSdk\Include\$taskSdkVersion\shared;$taskSdk\Include\$taskSdkVersion\um;$taskSdk\Include\$taskSdkVersion\winrt"
    $env:LIB = "$taskVc\lib\x64;$taskSdk\Lib\$taskSdkVersion\ucrt\x64;$taskSdk\Lib\$taskSdkVersion\um\x64"
    $env:VSCMD_ARG_HOST_ARCH = 'x64'
    $env:VSCMD_ARG_TGT_ARCH = 'x64'
    $env:VCToolsInstallDir = "$taskVc\"
    $env:WindowsSdkBinPath = "$taskSdk\bin\"
    $env:WindowsSDKVersion = "$taskSdkVersion\"
    $env:CC = 'cl.exe'
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = Join-Path $taskCompiler 'link.exe'
}
if (Test-Path -LiteralPath "$env:USERPROFILE\.cargo\bin") {
    $env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
}
if (Test-Path -LiteralPath "$taskRoot\.tools\bin") {
    $env:Path = "$taskRoot\.tools\bin;" + $env:Path
}
