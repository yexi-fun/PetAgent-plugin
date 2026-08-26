# Market metadata

`index.json` and `security/revocations.json` are generated release artifacts.
They are signed by protected keys during the GitHub Actions release workflow.
Do not replace signature fields with locally generated or placeholder values in
a production release.
