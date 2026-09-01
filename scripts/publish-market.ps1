param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [string]$Repository = "yexi-fun/PetAgent-plugin",
    [switch]$SkipRelease
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$archives = @(Get-ChildItem -LiteralPath (Join-Path $root "artifacts") -Recurse -Filter *.zip | Where-Object {
    $manifestPath = Join-Path $_.Directory.FullName "payload\manifest.json"
    if (-not (Test-Path $manifestPath)) { return $false }
    $staged = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
    $templatePath = Join-Path $root "plugins\$($staged.id)\manifest.template.json"
    if (-not (Test-Path $templatePath)) { return $false }
    $template = Get-Content -Raw -LiteralPath $templatePath | ConvertFrom-Json
    try { return ([version]$staged.version -le [version]$template.version) } catch { return $false }
})
if ($archives.Count -eq 0) { throw "No plugin ZIP was produced." }

$existingIndexPath = Join-Path $root "market\index.json"
$existingIndex = $null
if (Test-Path $existingIndexPath) {
    $existingIndex = Get-Content -Raw -LiteralPath $existingIndexPath | ConvertFrom-Json
}

$plugins = @()
$emptyFrontendCapabilities = [System.Collections.Generic.List[string]]::new()
foreach ($archive in $archives) {
    $artifactRoot = $archive.Directory.FullName
    $manifestPath = Join-Path $artifactRoot "payload\manifest.json"
    if (-not (Test-Path $manifestPath)) { throw "Missing staged manifest: $manifestPath" }
    $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
    $zipHash = (Get-FileHash -LiteralPath $archive.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $previous = @()
    if ($existingIndex -and $existingIndex.index -and $existingIndex.index.plugins) {
        $previous = @($existingIndex.index.plugins | Where-Object { $_.id -eq $manifest.id })
    }
    $categories = if ($previous.Count -gt 0 -and $previous[0].categories) {
        @($previous[0].categories)
    } else {
        @("Productivity")
    }
    if ($manifest.id -eq "com.petagent.hardware-temperature") {
        $categories = @("System")
    }
    $plugins += [pscustomobject]@{
        id = $manifest.id
        name = $manifest.name
        description = $manifest.description
        author = "PetAgent Plugin Team"
        type = $manifest.type
        categories = $categories
        permissions = @($manifest.permissions)
        dependencies = @($manifest.dependencies)
        conflicts = @($manifest.conflicts)
        frontendProtocolVersion = if ($manifest.type -in @("frontend", "app")) { $manifest.entry.frontend.protocolVersion } else { $null }
        frontendCapabilities = if ($manifest.type -in @("frontend", "app")) { @($manifest.entry.frontend.capabilities) } else { $emptyFrontendCapabilities }
        frontendWindow = if ($manifest.type -in @("frontend", "app")) { $manifest.entry.frontend.window } else { $null }
        serviceProtocolVersion = if ($manifest.type -eq "app") { $manifest.entry.service.protocolVersion } else { $null }
        agentCapabilities = if ($manifest.type -eq "app") { @($manifest.agent.capabilities) } else { @() }
        targets = @($manifest.targets)
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

# Multiple staged archives can represent different versions of one plugin.
# Keep one market entry per ID and merge all versions into that entry.
$mergedPlugins = @()
foreach ($group in ($plugins | Group-Object -Property id)) {
    $ordered = @($group.Group | Sort-Object { [version]$_.versions.version } -Descending)
    $first = $ordered[0]
    $versions = @($group.Group | ForEach-Object { $_.versions })
    $versions = @($versions | Sort-Object version)
    $mergedPlugin = [pscustomobject]@{
        id = $first.id
        name = $first.name
        description = $first.description
        author = $first.author
        type = $first.type
        categories = @($first.categories)
        permissions = $first.permissions
        dependencies = $first.dependencies
        conflicts = $first.conflicts
        frontendProtocolVersion = $first.frontendProtocolVersion
        frontendCapabilities = $first.frontendCapabilities
        frontendWindow = $first.frontendWindow
        serviceProtocolVersion = $first.serviceProtocolVersion
        agentCapabilities = $first.agentCapabilities
        targets = $first.targets
        versions = @($versions)
    }
    if ($first.type -notin @("frontend", "app")) {
        $mergedPlugin.PSObject.Properties.Remove("frontendProtocolVersion")
        $mergedPlugin.PSObject.Properties.Remove("frontendCapabilities")
        $mergedPlugin.PSObject.Properties.Remove("frontendWindow")
    }
    if ($first.type -ne "app") {
        $mergedPlugin.PSObject.Properties.Remove("serviceProtocolVersion")
        $mergedPlugin.PSObject.Properties.Remove("agentCapabilities")
    }
    $mergedPlugins += $mergedPlugin
}
$plugins = $mergedPlugins

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
