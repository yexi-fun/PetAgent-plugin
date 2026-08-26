# PetAgent Plugin Market

This repository is the controlled source for PetAgent marketplace plugins. It
contains reviewed plugin source, release metadata, signed market indexes and
revocation records.

## Layout

```text
plugins/<plugin-id>/       plugin source and package inputs
market/index.json          signed market index generated for a release
market/security/           signed revocation list
scripts/                   validation, packaging and signing helpers
.github/workflows/         pull-request and release gates
```

The client must consume an immutable tag or Release asset, never an unsigned
mutable branch file. Signing private keys belong in protected GitHub Actions
secrets and must never be committed here.

## Local checks

```powershell
pwsh ./scripts/validate-market.ps1
pwsh ./scripts/package-plugin.ps1 -PluginId com.example.echo
```

The package script refuses to publish until `MARKET_SIGNING_KEY` and
`PLUGIN_SIGNING_KEY` are supplied by the protected release environment.
