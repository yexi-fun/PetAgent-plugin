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

The client consumes the reviewed `yexi-fun/PetAgent-plugin` repository and
GitHub Release assets. This first-party market mode does not require signing
keys; CODEOWNERS review, protected branches, same-repository Release URLs and
SHA-256 checks remain required. Do not use this mode for an unreviewed or
third-party market.

## Local checks

```powershell
pwsh ./scripts/validate-market.ps1
pwsh ./scripts/package-plugin.ps1 -PluginId com.example.echo
pwsh ./scripts/publish-market.ps1 -Tag market-v1 -SkipRelease
```

The publish script generates an unsigned reviewed-repository index. Commit the
generated market files, push them to `main`, then create the matching tag and
Release asset.
