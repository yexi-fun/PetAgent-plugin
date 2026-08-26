# Contract checks

- `initialize`, `health`, `capabilities`, `tools/list`, `tools/call` and
  `shutdown` return JSON-RPC 2.0 responses with matching ids.
- Requests and responses are one line and bounded to 1 MiB.
- The package contains no absolute path, `..` component or symbolic link.
- The manifest is signed only by the protected release workflow.
