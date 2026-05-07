$ErrorActionPreference = 'Stop'

$Repo = 'Flutterando/statichost'
$Version = if ($env:STATICHOST_VERSION) { $env:STATICHOST_VERSION } else { 'latest' }
$InstallDir = Join-Path $env:USERPROFILE '.statichost'
$Exe = Join-Path $InstallDir 'statichost.exe'

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir | Out-Null
}

$arch = if ([Environment]::Is64BitOperatingSystem) {
    if ($env:PROCESSOR_ARCHITECTURE -match 'ARM64') { 'arm64' } else { 'amd64' }
} else { 'amd64' }

$bin = "statichost-windows-$arch.exe"

$url = if ($Version -eq 'latest') {
    "https://github.com/$Repo/releases/latest/download/$bin"
} else {
    "https://github.com/$Repo/releases/download/$Version/$bin"
}

Write-Host "Downloading $url"
Invoke-WebRequest -Uri $url -OutFile $Exe -UseBasicParsing

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$InstallDir", 'User')
    Write-Host "Added $InstallDir to user PATH (restart terminal)"
}

Write-Host "Installed: $Exe"
Write-Host "Run: statichost login --host https://your-server --token <your-token>"
