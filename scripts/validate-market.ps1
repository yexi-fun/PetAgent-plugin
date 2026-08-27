param(
    [string]$Repository = "yexi-fun/PetAgent-plugin"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$schemaPath = Join-Path $root "..\PetAgent\docs\插件开发\manifest.schema.json"
if (-not (Test-Path $schemaPath)) {
    $schemaPath = Join-Path $root "docs\manifest.schema.json"
}

function Read-Json([string]$Path) {
    Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
}

foreach ($path in Get-ChildItem -LiteralPath (Join-Path $root "market") -Recurse -Filter *.json) {
    Read-Json $path.FullName | Out-Null
}

$revocations = Read-Json (Join-Path $root "market\security\revocations.json")
if ($revocations.revocations.repository -ne $Repository) {
    throw "Revocation repository must be $Repository"
}

foreach ($manifestPath in Get-ChildItem -LiteralPath (Join-Path $root "plugins") -Recurse -Filter manifest.template.json) {
    $manifest = Read-Json $manifestPath.FullName
    if ($manifest.schemaVersion -ne 1 -or $manifest.apiVersion -ne 1) { throw "Unsupported manifest version: $($manifestPath.FullName)" }
    if ($manifest.id -notmatch '^[a-z0-9]+(?:[.-][a-z0-9]+)+$') { throw "Invalid plugin id: $($manifest.id)" }
    if ($manifest.type -ne $manifest.entry.kind) { throw "type and entry.kind differ: $($manifest.id)" }
    if ($manifest.type -eq "native-dll") {
        if ($manifest.entry.library -notmatch '\.dll$' -or $manifest.entry.abiVersion -ne 1 -or $manifest.entry.serviceName -ne "tools" -or $manifest.entry.serviceApiVersion -ne 1) { throw "Invalid native ABI entry: $($manifest.id)" }
    }
    if ($manifest.type -eq "frontend") {
        if (-not $manifest.entry.frontend.root -or -not $manifest.entry.frontend.index -or $manifest.entry.frontend.index -notlike "$($manifest.entry.frontend.root)/*") { throw "Invalid frontend entry: $($manifest.id)" }
    }
    foreach ($field in @("config", "executable", "library", "configSchema")) {
        $value = $manifest.entry.$field
        if ($null -eq $value) { $value = $manifest.$field }
        if ($value -and ($value -match '(^|[\\/])\.\.([\\/]|$)' -or [IO.Path]::IsPathRooted($value) -or $value.Contains('\'))) {
            throw "Unsafe relative path in $($manifest.id): $value"
        }
    }
    if ($manifest.entry.frontend) {
        foreach ($field in @("root", "index")) {
            $value = $manifest.entry.frontend.$field
            if ($value -and ($value -match '(^|[\\/])\.\.([\\/]|$)' -or [IO.Path]::IsPathRooted($value) -or $value.Contains('\\'))) {
                throw "Unsafe frontend relative path in $($manifest.id): $value"
            }
        }
    }
    if ($manifest.sha256 -and $manifest.sha256 -notmatch '^[0-9A-Fa-f]{64}$') { throw "Invalid optional sha256 field: $($manifest.id)" }
    # Signature and permission fields are descriptive metadata in trusted-code mode.
}

foreach ($archive in Get-ChildItem -LiteralPath (Join-Path $root "artifacts") -Recurse -Filter *.zip -ErrorAction SilentlyContinue) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($archive.FullName)
    try {
        foreach ($entry in $zip.Entries) {
            if ([IO.Path]::IsPathRooted($entry.FullName) -or $entry.FullName -match '(^|/)\.\.(/|$)' -or $entry.FullName -match '\\') {
                throw "Unsafe ZIP path: $($entry.FullName)"
            }
        }
    } finally { $zip.Dispose() }
}

Write-Host "Market metadata and archive paths are valid."
