# Fetches the two external tools video downloads need into a `tools` folder.
# Pookye DM looks in `tools` beside its executable first, then on PATH.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\fetch-tools.ps1 [-Dir <folder>]
#
# Existing files are left alone, so this is safe to run on every launch.

param([string]$Dir = "")

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

if (-not $Dir) {
    $Dir = Join-Path (Split-Path -Parent $PSScriptRoot) "target\release\tools"
}
New-Item -ItemType Directory -Force -Path $Dir | Out-Null

function Get-File($Url, $Path) {
    Write-Host "  downloading $(Split-Path -Leaf $Path)..."
    Invoke-WebRequest -Uri $Url -OutFile $Path -UseBasicParsing
}

$ytdlp = Join-Path $Dir "yt-dlp.exe"
if (-not (Test-Path $ytdlp)) {
    try {
        Get-File "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe" $ytdlp
    } catch {
        Write-Host "  yt-dlp could not be downloaded: $($_.Exception.Message)"
    }
}

$ffmpeg = Join-Path $Dir "ffmpeg.exe"
if (-not (Test-Path $ffmpeg)) {
    $temp = Join-Path ([System.IO.Path]::GetTempPath()) ("pookye-ffmpeg-" + [guid]::NewGuid())
    $zip = "$temp.zip"
    try {
        Get-File "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" $zip
        Expand-Archive -Path $zip -DestinationPath $temp -Force
        $built = Get-ChildItem -Path $temp -Recurse -Filter "ffmpeg.exe" | Select-Object -First 1
        if (-not $built) { throw "the archive held no ffmpeg.exe" }
        Copy-Item $built.FullName $ffmpeg -Force
    } catch {
        Write-Host "  ffmpeg could not be downloaded: $($_.Exception.Message)"
    } finally {
        Remove-Item $zip -Force -ErrorAction SilentlyContinue
        Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

foreach ($tool in @($ytdlp, $ffmpeg)) {
    if (Test-Path $tool) {
        $size = [math]::Round((Get-Item $tool).Length / 1MB, 1)
        Write-Host "  ok  $(Split-Path -Leaf $tool)  ${size} MB"
    } else {
        Write-Host "  missing $(Split-Path -Leaf $tool) -- video downloads will ask for it"
    }
}
