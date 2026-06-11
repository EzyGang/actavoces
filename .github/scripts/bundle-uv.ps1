$ErrorActionPreference = 'Stop'

$assetName = $env:UV_ASSET
$binaryName = $env:UV_BINARY
$assetUrl = "https://github.com/astral-sh/uv/releases/latest/download/$assetName"
$checksumUrl = "$assetUrl.sha256"
$releasePath = '.release/uv'
$archivePath = Join-Path $releasePath $assetName
$checksumPath = "$archivePath.sha256"
$extractPath = Join-Path $releasePath 'extracted'
$runtimePath = 'src-tauri/resources/runtime/uv'

New-Item -ItemType Directory -Force -Path $releasePath, $extractPath, $runtimePath | Out-Null
Invoke-WebRequest -Uri $assetUrl -OutFile $archivePath
Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath

$checksumContent = Get-Content $checksumPath -Raw
$expectedHash = [regex]::Match($checksumContent, '[a-fA-F0-9]{64}').Value.ToLowerInvariant()
$actualHash = (Get-FileHash -Algorithm SHA256 $archivePath).Hash.ToLowerInvariant()

if ($expectedHash -ne $actualHash) {
  throw "uv checksum mismatch for $assetName"
}

Expand-Archive -Path $archivePath -DestinationPath $extractPath -Force
$binary = Get-ChildItem -Path $extractPath -Recurse -File |
  Where-Object { $_.Name -eq $binaryName } |
  Select-Object -First 1

if ($null -eq $binary) {
  throw "Unable to find extracted uv binary: $binaryName"
}

Copy-Item -Path $binary.FullName -Destination (Join-Path $runtimePath $binaryName) -Force
& (Join-Path $runtimePath $binaryName) --version
