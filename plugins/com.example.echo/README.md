# Echo MCP plugin

This minimal plugin demonstrates the PetAgent MCP package contract. It
provides an `echo` tool over stdio and requires no permissions. The runtime
entry point is built for each Windows target by the release workflow.

The packaging script may record a payload SHA-256 as informational metadata.
PetAgent treats this plugin as trusted code and does not verify signatures or
enforce manifest permissions.
