# Contract

- `dist/index.html` is served only while the plugin is enabled.
- Parent paths, backslashes, external navigation and unauthorized IPC are rejected.
- `../__petagent/host-info.json` (resolved from `dist/index.html`) returns versioned handshake metadata and negotiated capabilities.
- `POST /__petagent/rpc` supports only manifest-granted host methods; the sample covers `config.get`, `config.set`, `window.getState` and `window.close`.
- The sample resolves `../__petagent/...` from `dist/index.html` so the plugin id remains in the custom-protocol URL.
