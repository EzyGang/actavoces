$ErrorActionPreference = 'Stop'

$assetName = $env:FFMPEG_ASSET
$binaryName = $env:FFMPEG_BINARY
$assetUrl = "https://github.com/Tyrrrz/FFmpegBin/releases/latest/download/$assetName"
$releasePath = '.release/ffmpeg'
$archivePath = Join-Path $releasePath $assetName
$extractPath = Join-Path $releasePath 'extracted'
$runtimePath = Join-Path 'src-tauri/resources/runtime/ffmpeg' $env:RUST_TARGET

New-Item -ItemType Directory -Force -Path $releasePath, $extractPath, $runtimePath | Out-Null
Invoke-WebRequest -Uri $assetUrl -OutFile $archivePath

Expand-Archive -Path $archivePath -DestinationPath $extractPath -Force
$binary = Get-ChildItem -Path $extractPath -Recurse -File |
  Where-Object { $_.Name -eq $binaryName } |
  Select-Object -First 1

if ($null -eq $binary) {
  throw "Unable to find extracted FFmpeg binary: $binaryName"
}

Copy-Item -Path $binary.FullName -Destination (Join-Path $runtimePath $binaryName) -Force
& (Join-Path $runtimePath $binaryName) -version
