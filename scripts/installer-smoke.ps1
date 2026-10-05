param([string]$Installer)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (!$Installer) { $Installer = Join-Path $repo 'src-tauri\target\release\bundle\nsis\XTools_0.2.0_x64-setup.exe' }
if (!(Test-Path -LiteralPath $Installer -PathType Leaf)) { throw 'Installer EXE not found' }
$fixture = Join-Path $repo ('.tools\installer-v2-smoke-' + [Guid]::NewGuid().ToString('N'))
$install = Join-Path $fixture 'Install With Spaces'
$profile = Join-Path $fixture 'AppData Isolated'
New-Item -ItemType Directory -Path $fixture,$profile -Force | Out-Null
$setup = Join-Path $fixture 'setup-test.exe'
Copy-Item -LiteralPath $Installer -Destination $setup
$desktop = Join-Path ([Environment]::GetFolderPath('DesktopDirectory')) 'XTools.lnk'
$programDir = Join-Path ([Environment]::GetFolderPath('Programs')) 'XTools'
$programLink = Join-Path $programDir 'XTools.lnk'
$oldProgramDir = Test-Path -LiteralPath $programDir
$shortcutBackup = @()
foreach ($path in @($desktop,$programLink)) {
  $entry = [pscustomobject]@{ Path=$path; Existed=(Test-Path -LiteralPath $path); Backup=(Join-Path $fixture ([Guid]::NewGuid().ToString('N') + '.lnk')) }
  if ($entry.Existed) { Copy-Item -LiteralPath $path -Destination $entry.Backup }
  $shortcutBackup += $entry
}
$regPaths = @('Software\XTools','Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools')
$runPath = 'Software\Microsoft\Windows\CurrentVersion\Run'
function Read-RegTree($key) {
  $vals = @()
  foreach ($name in $key.GetValueNames()) { $vals += [pscustomobject]@{Name=$name;Kind=$key.GetValueKind($name);Value=$key.GetValue($name,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)} }
  $subs = @()
  foreach ($name in $key.GetSubKeyNames()) { $child=$key.OpenSubKey($name); try { $subs += [pscustomobject]@{Name=$name;Tree=(Read-RegTree $child)} } finally {$child.Dispose()} }
  return [pscustomobject]@{Values=$vals;Children=$subs}
}
function Write-RegTree($key,$tree) {
  foreach($v in $tree.Values){$key.SetValue($v.Name,$v.Value,$v.Kind)}
  foreach($s in $tree.Children){$child=$key.CreateSubKey($s.Name);try{Write-RegTree $child $s.Tree}finally{$child.Dispose()}}
}
$registryBackup=@()
foreach($view in @([Microsoft.Win32.RegistryView]::Registry32,[Microsoft.Win32.RegistryView]::Registry64)){
  $hive=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,$view)
  try{
    foreach($path in $regPaths){$key=$hive.OpenSubKey($path);$tree=$null;if($null -ne $key){try{$tree=Read-RegTree $key}finally{$key.Dispose()}};$registryBackup += [pscustomobject]@{View=$view;Path=$path;Tree=$tree}}
    $key=$hive.OpenSubKey($runPath);$run=$null;if($null -ne $key){try{if($key.GetValueNames() -contains 'XTools'){$run=[pscustomobject]@{Value=$key.GetValue('XTools',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames);Kind=$key.GetValueKind('XTools')}}}finally{$key.Dispose()}};
    $registryBackup += [pscustomobject]@{View=$view;Path=$runPath;Run=$run}
  }finally{$hive.Dispose()}
}
$registryBackup | Export-Clixml -LiteralPath (Join-Path $fixture 'registry-backup.clixml')
$shortcutBackup | Export-Clixml -LiteralPath (Join-Path $fixture 'shortcut-backup.clixml')
$results=[System.Collections.Generic.List[object]]::new()
function Check($name,$ok,$details){$results.Add([pscustomobject]@{Name=$name;Passed=[bool]$ok;Details=$details});Write-Output ($name+': '+$ok+' '+$details)}
function Read-Key($path,$name,$view=[Microsoft.Win32.RegistryView]::Registry64){
  $hive=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,$view);try{$key=$hive.OpenSubKey($path);if($null -eq $key){return $null};try{return $key.GetValue($name,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)}finally{$key.Dispose()}}finally{$hive.Dispose()}
}
function Key-Exists($path,$view){$hive=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,$view);try{$key=$hive.OpenSubKey($path);if($null -eq $key){return $false};$key.Dispose();return $true}finally{$hive.Dispose()}}
function Set-Config($autostart){
  $dir=Join-Path $profile 'XTools';New-Item -ItemType Directory -Path $dir -Force|Out-Null
  $config=@{hotkey='Alt+Space';autostart=$autostart;clipboardLimit=100;showRecent=$true}|ConvertTo-Json
  [IO.File]::WriteAllText((Join-Path $dir 'config.json'),$config,[Text.UTF8Encoding]::new($false))
}
function Launch-Test {
  $prior=$env:APPDATA;$env:APPDATA=$profile
  try{$proc=Start-Process -FilePath (Join-Path $install 'xtools.exe') -ArgumentList '--autostart' -WindowStyle Hidden -PassThru}finally{$env:APPDATA=$prior}
  Start-Sleep -Seconds 3
  if($proc.HasExited){throw 'XTools test instance exited before readiness'}
  return $proc
}
function Quit-Test {
  $exe=Join-Path $install 'xtools.exe';if(!(Test-Path -LiteralPath $exe)){return}
  $prior=$env:APPDATA;$env:APPDATA=$profile
  try{$quit=Start-Process -FilePath $exe -ArgumentList '--quit' -WindowStyle Hidden -PassThru; $quit.WaitForExit(10000)|Out-Null}finally{$env:APPDATA=$prior}
  Start-Sleep -Milliseconds 800
}
function Wait-Uninstalled {
  for($i=0;$i -lt 30;$i++){if(!(Test-Path -LiteralPath (Join-Path $install 'xtools.exe'))){return};Start-Sleep -Milliseconds 200}
}
function Run-Installer($options){
  $args=$options + ' /D=' + $install
  $proc=Start-Process -FilePath $setup -ArgumentList $args -WindowStyle Hidden -PassThru -Wait
  Check 'installer exit' ($proc.ExitCode -eq 0) ('options='+$options+' exit='+$proc.ExitCode)
}
function Run-Uninstaller {
  $proc=Start-Process -FilePath (Join-Path $install 'Uninstall.exe') -ArgumentList '/S' -WindowStyle Hidden -PassThru -Wait
  Wait-Uninstalled;Start-Sleep -Milliseconds 800
  Check 'uninstaller exit' ($proc.ExitCode -eq 0) ('exit='+$proc.ExitCode)
}
$app=$null
try{
  Run-Installer '/S /NODESKTOP /NOAUTOSTART'
  Check 'custom D path with spaces' (Test-Path -LiteralPath (Join-Path $install 'xtools.exe')) $install
  $taskExpectedVersion = (Get-Content -LiteralPath (Join-Path $repo 'package.json') -Raw | ConvertFrom-Json).version
  $taskInstalledVersion = (Get-Item -LiteralPath (Join-Path $install 'xtools.exe')).VersionInfo.ProductVersion
  Check 'installed executable version' ($taskInstalledVersion -eq $taskExpectedVersion) $taskInstalledVersion
  Check 'no desktop selected' (!(Test-Path -LiteralPath $desktop)) 'desktop absent'
  Check 'start menu created' (Test-Path -LiteralPath $programLink) 'start menu present'
  $shell=[Activator]::CreateInstance([type]::GetTypeFromProgID('WScript.Shell'))
  $target=$shell.CreateShortcut($programLink).TargetPath
  Check 'start menu target' ($target -eq (Join-Path $install 'xtools.exe')) $target
  $taskShellMetadata = New-Object -ComObject Shell.Application
  $taskIdentity = $taskShellMetadata.Namespace($programDir).ParseName('XTools.lnk').ExtendedProperty('System.AppUserModel.ID')
  Check 'notification shortcut identity' ($taskIdentity -eq 'com.antique.xtools') $taskIdentity
  Check 'no startup selected' ($null -eq (Read-Key $runPath 'XTools')) 'Run XTools absent'
  Check 'installer opt-out marker' ((Read-Key 'Software\XTools' 'AutostartPreference') -eq 0 -and (Read-Key 'Software\XTools' 'ApplyInstallerPreference') -eq 1) 'preference=0 marker=1'
  [IO.File]::WriteAllText((Join-Path $install 'foreign-user-file.txt'),'must remain',[Text.UTF8Encoding]::new($false))
  Set-Config $true
  $app=Launch-Test
  $config=Get-Content -LiteralPath (Join-Path $profile 'XTools\config.json') -Raw|ConvertFrom-Json
  Check 'existing profile opt-out applied' (!$config.autostart -and $null -eq (Read-Key 'Software\XTools' 'ApplyInstallerPreference')) 'true became false; marker consumed'
  Quit-Test;Check 'quit command' $app.HasExited ('pid='+$app.Id);$app=$null
  Set-Config $true;$app=Launch-Test
  $config=Get-Content -LiteralPath (Join-Path $profile 'XTools\config.json') -Raw|ConvertFrom-Json
  Check 'preference applied only once' ($config.autostart -and (Read-Key $runPath 'XTools') -like ('*'+$install+'*')) 'manual true preserved on second launch'
  Quit-Test;$app=$null
  Run-Uninstaller
  Check 'stopped uninstall program removed' (!(Test-Path -LiteralPath (Join-Path $install 'xtools.exe'))) 'executable absent'
  foreach($view in @([Microsoft.Win32.RegistryView]::Registry32,[Microsoft.Win32.RegistryView]::Registry64)){
    Check ('uninstall registry removed '+$view) (!(Key-Exists $regPaths[0] $view) -and !(Key-Exists $regPaths[1] $view)) 'Software/XTools + Uninstall/XTools'
  }
  Check 'foreign file preserved' ((Get-Content -LiteralPath (Join-Path $install 'foreign-user-file.txt') -Raw) -eq 'must remain') 'nonrecursive uninstall'
  Check 'AppData preserved' ((Test-Path -LiteralPath (Join-Path $profile 'XTools\config.json')) -and (Test-Path -LiteralPath (Join-Path $profile 'XTools\data\xtools.db'))) 'config and database retained'
  Run-Installer '/S'
  Check 'default desktop checked' (Test-Path -LiteralPath $desktop) 'desktop shortcut present'
  $target=$shell.CreateShortcut($desktop).TargetPath
  Check 'desktop target' ($target -eq (Join-Path $install 'xtools.exe')) $target
  $runValue=Read-Key $runPath 'XTools'
  Check 'default startup checked' ($runValue -eq ('"'+$install+'\xtools.exe" --autostart')) $runValue
  $iconValue=Read-Key 'Software\Microsoft\Windows\CurrentVersion\Uninstall\XTools' 'DisplayIcon'
  Check 'uninstall icon target' ($iconValue -eq (Join-Path $install 'xtools.exe')) $iconValue
  Check 'default marker' ((Read-Key 'Software\XTools' 'AutostartPreference') -eq 1 -and (Read-Key 'Software\XTools' 'ApplyInstallerPreference') -eq 1) 'preference=1 marker=1'
  Set-Config $false;$app=Launch-Test
  $config=Get-Content -LiteralPath (Join-Path $profile 'XTools\config.json') -Raw|ConvertFrom-Json
  Check 'existing profile opt-in applied' ($config.autostart -and $null -eq (Read-Key 'Software\XTools' 'ApplyInstallerPreference')) 'false became true; marker consumed'
  $pidBefore=$app.Id
  Run-Uninstaller
  Start-Sleep -Milliseconds 500
  Check 'running instance quit by uninstall' $app.HasExited ('pid='+$pidBefore)
  if($app.HasExited){$app=$null}
  Check 'program removed' (!(Test-Path -LiteralPath (Join-Path $install 'xtools.exe'))) 'executable absent'
  Check 'shortcuts removed' (!(Test-Path -LiteralPath $desktop) -and !(Test-Path -LiteralPath $programLink)) 'desktop and start menu absent'
  Check 'Run removed' ($null -eq (Read-Key $runPath 'XTools')) 'own autostart removed'
  foreach($view in @([Microsoft.Win32.RegistryView]::Registry32,[Microsoft.Win32.RegistryView]::Registry64)){
    Check ('running uninstall registry removed '+$view) (!(Key-Exists $regPaths[0] $view) -and !(Key-Exists $regPaths[1] $view)) 'Software/XTools + Uninstall/XTools'
  }
  Check 'running uninstall preserves foreign file and data' ((Test-Path -LiteralPath (Join-Path $install 'foreign-user-file.txt')) -and (Test-Path -LiteralPath (Join-Path $profile 'XTools\config.json'))) 'foreign file + existing profile'
}finally{
  if($null -ne $app -and !$app.HasExited){Quit-Test}
  foreach($entry in $registryBackup){
    $hive=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,$entry.View)
    try{
      if($entry.Path -eq $runPath){$key=$hive.CreateSubKey($entry.Path);try{$key.DeleteValue('XTools',$false);if($null -ne $entry.Run){$key.SetValue('XTools',$entry.Run.Value,$entry.Run.Kind)}}finally{$key.Dispose()}}
      else{$hive.DeleteSubKeyTree($entry.Path,$false);if($null -ne $entry.Tree){$key=$hive.CreateSubKey($entry.Path);try{Write-RegTree $key $entry.Tree}finally{$key.Dispose()}}}
    }finally{$hive.Dispose()}
  }
  foreach($entry in $shortcutBackup){if($entry.Existed){New-Item -ItemType Directory -Path (Split-Path $entry.Path) -Force|Out-Null;Copy-Item -LiteralPath $entry.Backup -Destination $entry.Path -Force}else{if(Test-Path -LiteralPath $entry.Path){Remove-Item -LiteralPath $entry.Path -Force}}}
  if(!$oldProgramDir -and (Test-Path -LiteralPath $programDir) -and @(Get-ChildItem -LiteralPath $programDir -Force).Count -eq 0){Remove-Item -LiteralPath $programDir}
  $results|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $fixture 'results.json') -Encoding utf8
  Write-Output ('FIXTURE='+$fixture)
  Write-Output ('PACKAGE_SHA256='+(Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash)
  Write-Output 'Original registry and shortcut environment restored.'
}
