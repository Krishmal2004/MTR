$ErrorActionPreference = "Stop"

$Repo = "Krishmal2004/MTR"
$BinName = "tui-runner.exe"
$InstallDir = "$env:LOCALAPPDATA\Programs\tui-runner"
$Url = "https://github.com/$Repo/releases/latest/download/tui-runner-windows-x86_64.exe"

Write-Host "Downloading tui-runner from latest release..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$Dest = Join-Path $InstallDir $BinName
Invoke-WebRequest -Uri $Url -OutFile $Dest

Write-Host "Installed to $Dest"

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not ($UserPath -split ";" | Where-Object { $_ -eq $InstallDir })) {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to your user PATH. Restart your terminal to use it."
} else {
    Write-Host "$InstallDir is already on your PATH."
}

Write-Host "Run it with: tui-runner"
