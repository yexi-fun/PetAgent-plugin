param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [string]$Repository = "yexi-fun/PetAgent-plugin",
    [switch]$SkipRelease
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$archives = @(Get-ChildItem -LiteralPath (Join-Path $root "artifacts") -Recurse -Filter *.zip)
if ($archives.Count -eq 0) { throw "No plugin ZIP was produced." }

$plugins = @()
foreach ($archive in $archives) {
    $artifactRoot = $archive.Directory.FullName
    $manifestPath = Join-Path $artifactRoot "payload\manifest.json"
    if (-not (Test-Path $manifestPath)) { throw "Missing staged manifest: $manifestPath" }
    $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
    $zipHash = (Get-FileHash -LiteralPath $archive.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $plugins += [pscustomobject]@{
        id = $manifest.id
        name = $manifest.name
        description = $manifest.description
        author = "PetAgent Plugin Team"
        type = $manifest.type
        categories = @("Productivity")
        permissions = @($manifest.permissions)
        dependencies = @($manifest.dependencies)
        conflicts = @($manifest.conflicts)
        versions = @([pscustomobject]@{
            version = $manifest.version
            channel = "stable"
            downloadUrl = "https://github.com/$Repository/releases/download/$Tag/$($archive.Name)"
            sha256 = $zipHash
            publishedAt = (Get-Date).ToUniversalTime().ToString("o")
            revoked = $false
        })
    }
}

$index = [pscustomobject]@{
    index = [pscustomobject]@{
        schemaVersion = 1
        repository = $Repository
        generatedAt = (Get-Date).ToUniversalTime().ToString("o")
        plugins = $plugins
    }
}
$revocations = [pscustomobject]@{
    revocations = [pscustomobject]@{
        repository = $Repository
        versions = @()
    }
}
$utf8NoBom = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $root "market\index.json"), ($index | ConvertTo-Json -Depth 30), $utf8NoBom)
[IO.File]::WriteAllText((Join-Path $root "market\security\revocations.json"), ($revocations | ConvertTo-Json -Depth 20), $utf8NoBom)

Write-Host "Unsigned reviewed-repository index generated. Commit market/index.json before creating the tag."
if (-not $SkipRelease) {
    gh release create $Tag --repo $Repository --title "PetAgent market $Tag" --notes-file (Join-Path $root "docs\release.md") @($archives.FullName)
}
