# Contract

- The DLL exports exactly the ABI v1 functions and is built as `cdylib`.
- PetAgent does not require a DLL signature. It still rejects wrong-architecture,
  wrong-ABI and init-failing DLLs.
- Activation updates the lock and reports restart-required; loading occurs on next startup.
