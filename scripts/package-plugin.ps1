param(
    [Parameter(Mandatory = $true)][string]$PluginId,
    [string]$Version,
    [switch]$Release
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$pluginRoot = Join-Path $root "plugins\$PluginId"
if (-not (Test-Path $pluginRoot)) { throw "Unknown plugin: $PluginId" }
$templatePath = Join-Path $pluginRoot "manifest.template.json"
$manifest = Get-Content -Raw $templatePath | ConvertFrom-Json
if ($Version) { $manifest.version = $Version }
if ($Release -and ($env:PLUGIN_SIGNING_KEY -eq $null -or $env:MARKET_SIGNING_KEY -eq $null)) {
    throw "Release packaging requires protected PLUGIN_SIGNING_KEY and MARKET_SIGNING_KEY secrets."
}

$output = Join-Path $root "artifacts\$PluginId\$($manifest.version)"
New-Item -ItemType Directory -Force $output | Out-Null
$payload = Join-Path $output "payload"
if (Test-Path $payload) { Remove-Item -LiteralPath $payload -Recurse -Force }
New-Item -ItemType Directory -Force $payload | Out-Null
Copy-Item -LiteralPath (Join-Path $pluginRoot "README.md") -Destination $payload
Copy-Item -LiteralPath (Join-Path $pluginRoot "LICENSE") -Destination $payload
Copy-Item -LiteralPath (Join-Path $pluginRoot "config.schema.json") -Destination $payload
New-Item -ItemType Directory -Force (Join-Path $payload "runtime") | Out-Null
Copy-Item -LiteralPath (Join-Path $pluginRoot "runtime\mcp.json") -Destination (Join-Path $payload "runtime")

$binary = Join-Path $pluginRoot "runtime\windows-x64\echo-mcp.exe"
if (-not (Test-Path $binary)) {
    New-Item -ItemType Directory -Force (Split-Path -Parent $binary) | Out-Null
    Push-Location (Join-Path $pluginRoot "src")
    cargo build --release --target x86_64-pc-windows-msvc
    Pop-Location
    $built = Join-Path $pluginRoot "src\target\x86_64-pc-windows-msvc\release\echo-mcp.exe"
    if (Test-Path $built) { Copy-Item $built $binary }
}
if (-not (Test-Path $binary)) { throw "Missing runtime binary: build echo-mcp.exe before packaging." }

New-Item -ItemType Directory -Force (Join-Path $payload "runtime\windows-x64") | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $payload "runtime\windows-x64") -Force
# The host resolves this command from the extracted version directory.
# The release signer replaces the template digest with the deterministic
# payload digest and signs the normalized manifest after the package is staged.
function Get-PayloadSha256([string]$PayloadRoot) {
    $hash = [Security.Cryptography.IncrementalHash]::CreateHash([Security.Cryptography.HashAlgorithmName]::SHA256)
    $utf8 = [Text.Encoding]::UTF8
    try {
        $files = Get-ChildItem -LiteralPath $PayloadRoot -Recurse -File |
            Where-Object { $_.Name -ne "manifest.json" } |
            ForEach-Object {
                $relative = $_.FullName.Substring($PayloadRoot.Length).TrimStart('\', '/') -replace '\\', '/'
                [PSCustomObject]@{ Name = $relative; Path = $_.FullName }
            } |
            Sort-Object Name
        foreach ($file in $files) {
            $nameBytes = $utf8.GetBytes($file.Name)
            $content = [IO.File]::ReadAllBytes($file.Path)
            $hash.AppendData([BitConverter]::GetBytes([int64]$nameBytes.Length))
            $hash.AppendData($nameBytes)
            $hash.AppendData([BitConverter]::GetBytes([int64]$content.Length))
            $hash.AppendData($content)
        }
        return ([BitConverter]::ToString($hash.GetHashAndReset()) -replace '-', '').ToLowerInvariant()
    } finally { $hash.Dispose() }
}
$payloadHash = Get-PayloadSha256 $payload
$manifest.sha256 = $payloadHash
$manifest.signature = "GENERATED_BY_PROTECTED_RELEASE_WORKFLOW"
$manifestJson = $manifest | ConvertTo-Json -Depth 20
$manifestPath = Join-Path $payload "manifest.json"
$utf8NoBom = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText($manifestPath, $manifestJson, $utf8NoBom)

$archivePath = Join-Path $output "$PluginId-$($manifest.version).zip"
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
if (Test-Path $archivePath) { Remove-Item -LiteralPath $archivePath -Force }
$archiveStream = [IO.File]::Open($archivePath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
$archive = [IO.Compression.ZipArchive]::new($archiveStream, [IO.Compression.ZipArchiveMode]::Create, $false)
try {
    foreach ($file in Get-ChildItem -LiteralPath $payload -Recurse -File) {
        $relative = $file.FullName.Substring($payload.Length).TrimStart('\', '/') -replace '\\', '/'
        $entry = $archive.CreateEntry($relative, [IO.Compression.CompressionLevel]::Optimal)
        $input = [IO.File]::OpenRead($file.FullName)
        $outputStream = $entry.Open()
        try { $input.CopyTo($outputStream) } finally { $outputStream.Dispose(); $input.Dispose() }
    }
} finally {
    $archive.Dispose()
    $archiveStream.Dispose()
}
Write-Host "Package staged at $output"
