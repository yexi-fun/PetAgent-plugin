# Echo MCP plugin

This minimal plugin demonstrates the PetAgent MCP package contract. It
provides an `echo` tool over stdio and requires no permissions. The runtime
entry point is built for each Windows target by the release workflow.

Before a release, the packaging script computes the payload SHA-256 and signs
the normalized manifest. The signed market index then records the ZIP SHA-256,
Release URL and publisher public key.
