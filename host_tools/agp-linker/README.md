# agp-linker

This is a custom linker used by the firmware build process (see the [.cargo/config](../../firmware/.cargo/config)).

It's a simple wrapper around [flip-link](https://crates.io/crates/flip-link) that
does normal linking, followed by special linking to produce an firmware update archive that
can be used to perform a FOTA update using the [air-gradient-cli](../air-gradient-cli/README.md).

* Rust support for PIE isn't usable yet for the `thumbv7em-none-eabihf` target
* Building the firmware runs a script at link-time to produce
  two ELF binaries: one for each linked slot location in FLASH (0x0801_0000 and 0x0804_0000)
* The two binaries will be archived into a CPIO file by agp-linker
* Host tooling (CLI) will communicate with the application
  to determine which firmware slot is available for writting
* Host tooling (CLI) will extract the selected ELF from the arhive and
  convert it to binary (in-memory) before uploading in to the target

![fw_archive_build.png](../../doc/fw_archive_build.png)
