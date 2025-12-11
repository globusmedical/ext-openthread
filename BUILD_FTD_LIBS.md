# Building FTD Libraries for nRF Targets

This document describes how to build the Full Thread Device (FTD) libraries for nRF targets.

## Prerequisites

1. **ARM GCC Toolchain**: Install the ARM GNU toolchain from https://developer.arm.com/Tools%20and%20Software/GNU%20Toolchain
   - Ensure `arm-none-eabi-gcc` is in your PATH
   - Verify with: `arm-none-eabi-gcc --version`
   - Verify sysroot: `arm-none-eabi-gcc -print-sysroot` (should return a valid path)

2. **CMake**: Version 3.16 or later
   - Download from https://cmake.org/download/
   - Verify with: `cmake --version`

3. **Ninja**: Build system
   - Download from https://github.com/ninja-build/ninja/releases
   - Verify with: `ninja --version`

4. **Clang**: For bindgen
   - Download from https://releases.llvm.org/
   - Verify with: `clang --version`

5. **Rust**: With the appropriate targets
   ```bash
   rustup target add thumbv7em-none-eabi thumbv6m-none-eabi
   ```

## Building FTD Libraries

### For thumbv7em-none-eabi (nRF52840 and similar)

```bash
# From the repository root
cargo xtask gen --ftd thumbv7em-none-eabi
```

This will:
1. Build OpenThread in FTD mode using CMake
2. Generate Rust bindings including FTD-specific APIs
3. Copy the libraries to `openthread-sys/libs/thumbv7em-none-eabi/`
4. Copy the bindings to `openthread-sys/src/include/thumbv7em-none-eabi.rs`

The following libraries will be generated:
- `libopenthread-ftd.a` - Main FTD library
- `libopenthread-cli-ftd.a` - CLI support
- `libopenthread-ncp-ftd.a` - NCP support  
- `libtcplp-ftd.a` - TCP support
- Plus all common libraries (mbedtls, platform utils, etc.)

### For thumbv6m-none-eabi (nRF51 series)

```bash
cargo xtask gen --ftd thumbv6m-none-eabi
```

## Verifying the Build

After building, verify the libraries exist:

```bash
ls -la openthread-sys/libs/thumbv7em-none-eabi/libopenthread-ftd.a
ls -la openthread-sys/libs/thumbv6m-none-eabi/libopenthread-ftd.a
```

## Using FTD Libraries

To build your application with FTD support:

```bash
cargo build --target thumbv7em-none-eabi --features ftd
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
openthread = { version = "0.1", features = ["ftd", "embassy-nrf"] }
```

## Troubleshooting

### "arm-none-eabi-gcc: command not found"

Install the ARM GNU toolchain and add it to your PATH.

### "arm-none-eabi-gcc -print-sysroot returns empty"

The Ubuntu system package `arm-none-eabi-gcc` doesn't work. Use the official ARM GNU toolchain instead.

### CMake errors about missing compiler

Ensure the ARM GCC toolchain is properly installed and in your PATH.

### Bindgen errors

Ensure Clang is installed and the LLVM tools are in your PATH.

## Memory Requirements

FTD devices require significantly more RAM than MTD devices:

- **MTD**: ~30-50 KB RAM
- **FTD**: ~80-150 KB RAM (depending on configuration)

Configure the maximum number of children appropriately for your device:

```rust
// In your application
ot.set_max_allowed_children(10)?; // Default is usually 32
```

## Next Steps

After building the libraries:
1. See the examples in `examples/nrf/` for usage
2. Read the main README for FTD-specific API documentation
3. Configure your Thread network parameters in your application
