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
$output = Join-Path $root "artifacts\$PluginId\$($manifest.version)"
New-Item -ItemType Directory -Force $output | Out-Null
$payload = Join-Path $output "payload"
if (Test-Path $payload) { Remove-Item -LiteralPath $payload -Recurse -Force }
New-Item -ItemType Directory -Force $payload | Out-Null
Copy-Item -LiteralPath (Join-Path $pluginRoot "README.md") -Destination $payload
Copy-Item -LiteralPath (Join-Path $pluginRoot "LICENSE") -Destination $payload
Copy-Item -LiteralPath (Join-Path $pluginRoot "config.schema.json") -Destination $payload
if ($manifest.type -eq "mcp") {
    New-Item -ItemType Directory -Force (Join-Path $payload "runtime") | Out-Null
    Copy-Item -LiteralPath (Join-Path $pluginRoot "runtime\mcp.json") -Destination (Join-Path $payload "runtime")
    $mcpConfig = Get-Content -Raw -LiteralPath (Join-Path $pluginRoot "runtime\mcp.json") | ConvertFrom-Json
    [array]$servers = if ($mcpConfig.servers) { $mcpConfig.servers } else { $mcpConfig }
    if ($servers.Count -ne 1 -or $servers[0].transport -ne "stdio" -or -not $servers[0].command) {
        throw "MCP packaging currently requires exactly one stdio server with a command."
    }
    $command = [string]$servers[0].command
    $binaryName = [IO.Path]::GetFileName($command)
    $binary = Join-Path $pluginRoot ($command -replace '/', '\')
    New-Item -ItemType Directory -Force (Split-Path -Parent $binary) | Out-Null
    Push-Location (Join-Path $pluginRoot "src")
    try { cargo build --release --target x86_64-pc-windows-msvc } finally { Pop-Location }
    $built = Join-Path $pluginRoot "src\target\x86_64-pc-windows-msvc\release\$binaryName"
    if (Test-Path $built) { Copy-Item $built $binary -Force }
    if (-not (Test-Path $binary)) { throw "Missing runtime binary: build $binaryName before packaging." }
    New-Item -ItemType Directory -Force (Join-Path $payload "runtime\windows-x64") | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $payload "runtime\windows-x64") -Force
} elseif ($manifest.type -eq "frontend") {
    Copy-Item -LiteralPath (Join-Path $pluginRoot "dist") -Destination $payload -Recurse -Force
} elseif ($manifest.type -eq "native-dll") {
    $library = [string]$manifest.entry.library
    if (-not $library -or [IO.Path]::IsPathRooted($library) -or $library.Contains('\') -or $library -match '(^|/)\.\.(/|$)' -or [IO.Path]::GetExtension($library) -ne ".dll") {
        throw "Native DLL entry.library must be a safe relative .dll path."
    }
    Push-Location (Join-Path $pluginRoot "src")
    try {
        $cargoMetadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
        if ($LASTEXITCODE -ne 0) { throw "Failed to read Cargo metadata for $PluginId." }
        $cdylibTargets = @(
            foreach ($package in $cargoMetadata.packages) {
                foreach ($target in $package.targets) {
                    if (@($target.kind) -contains "cdylib") { $target }
                }
            }
        )
        if ($cdylibTargets.Count -ne 1) {
            throw "Native DLL packaging requires exactly one Cargo cdylib target."
        }
        cargo build --release --target x86_64-pc-windows-msvc
        if ($LASTEXITCODE -ne 0) { throw "Failed to build native DLL for $PluginId." }
    } finally { Pop-Location }
    $built = Join-Path ([string]$cargoMetadata.target_directory) "x86_64-pc-windows-msvc\release\$($cdylibTargets[0].name).dll"
    if (-not (Test-Path $built)) { throw "Missing runtime binary: $($cdylibTargets[0].name).dll" }
    $payloadBinary = Join-Path $payload ($library -replace '/', '\')
    New-Item -ItemType Directory -Force (Split-Path -Parent $payloadBinary) | Out-Null
    Copy-Item -LiteralPath $built -Destination $payloadBinary -Force
} else {
    throw "Unsupported plugin type: $($manifest.type)"
}
# The host resolves this command from the extracted version directory.
# The reviewed-repository release may keep a deterministic payload digest as
# informational metadata. Signature fields are not generated or trusted.
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
        $entry.LastWriteTime = [DateTimeOffset]::new(1980, 1, 1, 0, 0, 0, [TimeSpan]::Zero)
        $input = [IO.File]::OpenRead($file.FullName)
        $outputStream = $entry.Open()
        try { $input.CopyTo($outputStream) } finally { $outputStream.Dispose(); $input.Dispose() }
    }
} finally {
    $archive.Dispose()
    $archiveStream.Dispose()
}
Write-Host "Package staged at $output"
