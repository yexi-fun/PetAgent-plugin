# Release checklist

1. Update `manifest.template.json`, changelog and runtime binaries for every
   supported target.
2. Run `scripts/validate-market.ps1` and the plugin contract tests.
3. In the protected `market-release` environment, provide Ed25519
   `PLUGIN_SIGNING_KEY`, `MARKET_SIGNING_KEY` and the approved signer path
   `MARKET_SIGNER_PATH`. Do not put keys in repository variables or files.
4. Create an immutable `market-v<version>` tag. The release workflow must
   compute the payload digest, sign the normalized manifest, compute the ZIP
   digest, update `market/index.json` and sign both the index and revocations.
5. Publish the ZIP under the GitHub Release path expected by the client:
   `/yexi-fun/PetAgent-plugin/releases/download/<tag>/<plugin-id>/<version>/<plugin-id>.zip`.
6. Verify a clean PetAgent installation can refresh the signed index, install,
   enable, call and disable the plugin.

The host currently ships with its default source configured separately. After
this repository is published, set the host source to
`https://raw.githubusercontent.com/yexi-fun/PetAgent-plugin/<tag>/market/index.json`
and build the release with the matching trusted market public key.
