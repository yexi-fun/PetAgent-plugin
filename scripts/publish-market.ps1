param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [string]$Repository = "yexi-fun/PetAgent-plugin"
)

$ErrorActionPreference = "Stop"
if (-not $env:MARKET_SIGNING_KEY -or -not $env:PLUGIN_SIGNING_KEY) {
    throw "MARKET_SIGNING_KEY and PLUGIN_SIGNING_KEY must be provided by the protected environment."
}
if (-not $env:MARKET_SIGNER_PATH) {
    throw "Configure MARKET_SIGNER_PATH to the approved Ed25519 signing tool in the protected runner."
}

$root = Split-Path -Parent $PSScriptRoot
& $env:MARKET_SIGNER_PATH `
    --repository $Repository `
    --tag $Tag `
    --root $root `
    --market-key $env:MARKET_SIGNING_KEY `
    --plugin-key $env:PLUGIN_SIGNING_KEY
if ($LASTEXITCODE -ne 0) { throw "Market signer failed with exit code $LASTEXITCODE" }

$assets = Get-ChildItem -LiteralPath (Join-Path $root "artifacts") -Recurse -Filter *.zip
if (-not $assets) { throw "No plugin ZIP was produced." }
gh release create $Tag --repo $Repository --title "PetAgent market $Tag" --notes-file (Join-Path $root "docs\release.md") @($assets.FullName)
