# Kwen Installer for Windows
# Run with: powershell -ExecutionPolicy Bypass -File install.ps1

$ErrorActionPreference = "Stop"

$kwendir = "$env:USERPROFILE\kwen"
$binary  = "$PSScriptRoot\target\release\kwen.exe"
$dest    = "$kwendir\kwen.exe"
$config_dir = "$env:APPDATA\..\Local\kwen"   # %LOCALAPPDATA%\kwen

Write-Host ""
Write-Host "  [38;2;16;185;129m╭─────────────────────────────────────────╮[0m"
Write-Host "  [38;2;16;185;129m│[0m  KWEN Installer  v0.1.0                [38;2;16;185;129m│[0m"
Write-Host "  [38;2;16;185;129m╰─────────────────────────────────────────╯[0m"
Write-Host ""

# 1. Check binary exists (must build first)
if (-not (Test-Path $binary)) {
    Write-Host "  [38;2;239;68;68m[ERROR][0m Binary not found at $binary"
    Write-Host "  Please run  cargo build --release  first."
    exit 1
}

# 2. Create install directory
if (-not (Test-Path $kwendir)) {
    New-Item -ItemType Directory -Force -Path $kwendir | Out-Null
    Write-Host "  [38;2;52;211;153m[OK][0m Created install directory: $kwendir"
}

# 3. Copy binary
Copy-Item -Force $binary $dest
Write-Host "  [38;2;52;211;153m[OK][0m Installed kwen.exe  →  $dest"

# 4. Write default config.json
$config_dir_path = "$env:USERPROFILE\AppData\Roaming\kwen"
if (-not (Test-Path $config_dir_path)) {
    New-Item -ItemType Directory -Force -Path $config_dir_path | Out-Null
}

$config_file = "$config_dir_path\config.json"
if (-not (Test-Path $config_file)) {
    $default_config = @{
        base_url        = "http://localhost:1234/v1"
        model           = "qwen2.5-coder-7b-instruct"
        temperature     = 0.2
        max_tokens      = 4096
        system_prompt   = $null
        default_context = $null
    } | ConvertTo-Json -Depth 5
    Set-Content -Path $config_file -Value $default_config -Encoding UTF8
    Write-Host "  [38;2;52;211;153m[OK][0m Created default config  →  $config_file"
} else {
    Write-Host "  [38;2;245;158;11m[SKIP][0m Config already exists at $config_file"
}

# 5. Add to User PATH if not already there
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$kwendir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$kwendir", "User")
    Write-Host "  [38;2;52;211;153m[OK][0m Added $kwendir to your PATH"
    Write-Host ""
    Write-Host "  [38;2;245;158;11mIMPORTANT:[0m Restart your terminal for 'kwen' to work from anywhere."
} else {
    Write-Host "  [38;2;52;211;153m[OK][0m PATH already contains $kwendir"
}

Write-Host ""
Write-Host "  [38;2;52;211;153m╭─────────────────────────────────────────╮[0m"
Write-Host "  [38;2;52;211;153m│[0m  Installation complete!                 [38;2;52;211;153m│[0m"
Write-Host "  [38;2;52;211;153m│[0m                                         [38;2;52;211;153m│[0m"
Write-Host "  [38;2;52;211;153m│[0m  Run:  kwen                             [38;2;52;211;153m│[0m"
Write-Host "  [38;2;52;211;153m│[0m  Config: $config_file  [38;2;52;211;153m│[0m"
Write-Host "  [38;2;52;211;153m╰─────────────────────────────────────────╯[0m"
Write-Host ""
