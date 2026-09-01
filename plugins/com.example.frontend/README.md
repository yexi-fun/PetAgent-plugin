# Frontend sample

This sample is served from a PetAgent WebView through the `pet-plugin` protocol.
Frontend plugins are trusted code and run with the host user's permissions;
the protocol maps packaged resources, negotiated host capabilities, plugin-scoped
configuration RPC, window state/close, notifications, and host lifecycle events.
