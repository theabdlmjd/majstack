$ErrorActionPreference = "Stop"

$Repo = if ($env:MAJSTACK_REPO) { $env:MAJSTACK_REPO } else { "theabdlmjd/majstack" }
$Version = if ($env:MAJSTACK_VERSION) { $env:MAJSTACK_VERSION } else { "latest" }
$BinDir = if ($env:MAJSTACK_BIN_DIR) { $env:MAJSTACK_BIN_DIR } else { Join-Path $env:USERPROFILE ".majstack\bin" }

$Asset = "majstack-x86_64-pc-windows-msvc.zip"
if ($Version -eq "latest") {
    $Url = "https://github.com/$Repo/releases/latest/download/$Asset"
} else {
    $Url = "https://github.com/$Repo/releases/download/$Version/$Asset"
}

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

try {
    $Tmp = Join-Path $env:TEMP "majstack-install.zip"
    Invoke-WebRequest -Uri $Url -OutFile $Tmp -UseBasicParsing
    Expand-Archive -Path $Tmp -DestinationPath $BinDir -Force
    Write-Host "installed majstack to $BinDir"
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$BinDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$BinDir", "User")
        Write-Host "added $BinDir to your PATH (open a new terminal)"
    }
    exit 0
} catch {
    Write-Host "release download unavailable; trying cargo"
}

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    cargo install --git "https://github.com/$Repo" majstack-cli majstack-daemon
    Write-Host "installed via cargo to $env:USERPROFILE\.cargo\bin"
    exit 0
}

throw "Download failed and cargo is not installed. Install Rust from https://rustup.rs or download from https://github.com/$Repo/releases"
