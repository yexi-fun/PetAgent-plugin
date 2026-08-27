# Release checklist

1. Update `manifest.template.json`, changelog and runtime binaries for every
   supported target.
2. Run `scripts/validate-market.ps1` and the plugin contract tests.
3. Run `scripts/publish-market.ps1 -Tag <tag> -SkipRelease` to compute payload
   and ZIP hashes and regenerate the reviewed-repository index.
4. Review and commit `market/index.json` and
   `market/security/revocations.json`, then push `main`.
5. Create the matching immutable tag and publish the ZIP at the GitHub Release
   URL recorded in the index.
6. Verify a clean PetAgent installation can refresh the reviewed index, install,
   enable, call and disable the plugin.

The host currently ships with its default source configured separately. After
this repository is published, set the host source to
`https://raw.githubusercontent.com/yexi-fun/PetAgent-plugin/<tag>/market/index.json`
and build the release with the matching trusted market public key.
