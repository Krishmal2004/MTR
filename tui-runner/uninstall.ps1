$ErrorActionPreference = "Stop"

$InstallDir = "$env:LOCALAPPDATA\Programs\tui-runner"
$BinPath = Join-Path $InstallDir "tui-runner.exe"

if (Test-Path $BinPath) {
    Remove-Item -Force $BinPath
    Write-Host "Removed $BinPath"
} else {
    Write-Host "Nothing to remove at $BinPath (already uninstalled?)"
}

if ((Test-Path $InstallDir) -and ((Get-ChildItem $InstallDir -Force | Measure-Object).Count -eq 0)) {
    Remove-Item -Force $InstallDir
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath) {
    $Entries = $UserPath -split ";" | Where-Object { $_ -ne "" -and $_ -ne $InstallDir }
    $NewPath = ($Entries -join ";")
    if ($NewPath -ne $UserPath) {
        [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        Write-Host "Removed $InstallDir from your user PATH. Restart your terminal for it to take effect."
    }
}
