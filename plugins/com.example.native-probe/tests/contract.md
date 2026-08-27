# Contract

- The DLL exports exactly the ABI v1 functions and is built as `cdylib`.
- PetAgent must reject unsigned, wrong-architecture, wrong-ABI and init-failing DLLs.
- Activation updates the lock and reports restart-required; loading occurs on next startup.
