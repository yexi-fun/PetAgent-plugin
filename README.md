# PetAgent Plugin Market

This repository is the controlled source for PetAgent marketplace plugins. It
contains reviewed plugin source and release metadata for trusted-code plugins.

## Layout

```text
plugins/<plugin-id>/       plugin source and package inputs
market/index.json          market index generated for a release
market/security/           optional manual takedown metadata
scripts/                   validation and packaging helpers
.github/workflows/         pull-request and release gates
```

The client consumes the reviewed `yexi-fun/PetAgent-plugin` repository and
GitHub Release assets. PetAgent treats installed plugins as trusted code: it
does not verify signatures, provide a sandbox, or enforce manifest permissions.
CODEOWNERS review, protected branches and same-repository Release URLs are
operational controls only. Use this repository for sources you trust.

## Local checks

```powershell
pwsh ./scripts/validate-market.ps1
pwsh ./scripts/package-plugin.ps1 -PluginId com.example.echo
pwsh ./scripts/publish-market.ps1 -Tag market-v1 -SkipRelease
```

The publish script generates a reviewed-repository index without cryptographic
signing. Commit the generated market files, push them to `main`, then create the
matching tag and Release asset.
