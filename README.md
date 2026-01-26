# openthread

[![CI](https://github.com/esp-rs/esp-openthread/actions/workflows/rust_ci.yml/badge.svg)](https://github.com/esp-rs/esp-openthread/actions/workflows/rust_ci.yml)
[![crates.io](https://img.shields.io/crates/v/openthread.svg)](https://crates.io/crates/openthread)
[![Documentation](https://img.shields.io/badge/docs-esp--rs-brightgreen)](https://esp-rs.github.io/esp-rs/esp-openthread/index.html)
[![Matrix](https://img.shields.io/matrix/esp-rs:matrix.org?label=join%20matrix&color=BEC5C9&logo=matrix)](https://matrix.to/#/#esp-rs:matrix.org)

Platform-agnostic, async Rust bindings for the [`OpenThread`](https://openthread.io/) library.

Tailored for Rust embedded baremetal.

The [crate](openthread) does not depend on any platform features and only needs an implementation of a single trait - [`Radio`](openthread/src/radio.rs) - that represents the IEEE 802.15.4 PHY radio. 
The radio might be located on the same die, or the user might provide an implementation that communicates with the actual radio over UART, SPI, USB, etc.

Two IEEE 802.15.4 radios are supported out of the box:
- The ESP32C6 and ESP32H2 radio (enable the `esp-ieee802154` feature; see [examples](examples/esp));
- The NRF radio (enable the `embasy-nrf` feature; see [examples](examples/nrf)).

## Build

For certain MCUs / Rust targets, the OpenThread libraries are pre-compiled for convenience.
Current list (might be extended upon request):
- `riscv32imac-unknown-none-elf` (ESP32C6 and ESP32H2)
- `thumbv7em-none-eabi` (NRF52)
- `thumbv6m-none-eabi` No chip with IEEE802.15.4 radio on this target (to our knowledge), but can be used with `openthread` RCP (radio offloading)

**For these targets you only need `rustc`/`cargo` as usual!**

Small caveat: since `openthread` does a few calls into the C standard library (primarily `str*` functions), at link time, it is up to the user to poly-fill the `str*` syscalls - [either with the MCU ROM functions](examples/esp/.cargo/config.toml), or by [depending](examples/nrf/Cargo.toml) on [`tinyrlibc`](https://github.com/rust-embedded-community/tinyrlibc), or with both.

### Build for other targets / custom build

For targets where pre-compiled libs are not available (including for the Host itself), a standard `build.rs` build is also supported.
For the on-the-fly OpenThread CMake build to work, you'll need to install and set in your `$PATH`:
- The GCC toolchain correspponding to your Rust target, with **working** `foo-bar-gcc -print-sysroot` command
- Recent Clang (for Espressif `xtensa`, [it must be the Espressif fork](https://crates.io/crates/espup), but for all other chips, the stock Clang would work)
- CMake and Ninja

As per above, since `openthread` does a few calls into the C standard library (primarily `str*` functions), the GCC toolchain needs to have the `newlib` (or other libc headers for e.g. non-embedded scenarios) in its sysroot, which is usually the case anyway. `newlib` however is only used _at compile-time_ on baremetal targets (for a few libc headers) and not linked-in.

Examples of GCC toolchains that are known to work fine:
- For ARM (Cortex M CPUs and others) - the [ARM GNU toolchain](https://developer.arm.com/Tools%20and%20Software/GNU%20Toolchain);
  - Note that the Ubuntu `arm-none-eabi-gcc` system package does **NOT** work, as it does not print a sysroot, i.e. `arm-none-eabi-gcc -print-sysroot` returns an empty response, and furthermore, the `newlib` headers are installed in a separate location from the arch headers;
- For RISCV - the [Espressif RISCV toolchain](https://github.com/espressif/crosstool-NG/releases). The ["official" RISCV GNU toolchain](https://github.com/riscv-collab/riscv-gnu-toolchain) should also work;
- For xtensa (Espressif ESP32 and ESP32SXX MCUs) - the [Espressif xtensa toolchain](https://github.com/espressif/crosstool-NG/releases);
- For the Host machine (non-embedded) - your pre-installed toolchain would work just fine.

## Features

- **MTD (Minimal Thread Device) functionality** - Default mode, low memory footprint
- **FTD (Full Thread Device) functionality** - Optional feature, enables Router/Leader roles and mesh networking
- Optional integration with [`embassy-net`]() and [`edge-nal`]()
- Out of the box support for the IEEE 802.15.4 radio in [Espressif](openthread/src/esp.rs) and [Nordic Semiconductor](openthread/src/nrf.rs) chips

### FTD vs MTD

- **MTD (Minimal Thread Device)**: Default mode, acts as an end device (child) in the network. Lower memory usage (~30-50KB RAM). Cannot route traffic or accept child devices.
- **FTD (Full Thread Device)**: Can act as Router or Leader, form mesh networks, and accept child devices. Requires more memory (~80-150KB RAM). Enable with the `ftd` feature flag.

## Next

- Sleepy end-device

## Non-Goals

- Thread Border Router functionality

## Status

The examples (native OpenThread UDP sockets; `embassy-net` integration; SRP) build and run on Espressif MCUs, and on the NRF52840.

Both MTD (Minimal Thread Device) and FTD (Full Thread Device) modes are supported.

## Using FTD Mode

To build with FTD (Full Thread Device) support instead of the default MTD mode:

### In your Cargo.toml

```toml
[dependencies]
openthread = { version = "0.1", features = ["ftd", "embassy-nrf"] }
```

### Building Examples

```bash
# Build nRF FTD example
cd examples/nrf
cargo build --release --bin ftd_basic --features ftd

# Flash to nRF52840-DK
probe-rs run --chip nRF52840_xxAA --release --bin ftd_basic --features ftd
```

### FTD-Specific APIs

When the `ftd` feature is enabled, additional APIs become available:

```rust
// Iterate over child devices
ot.children(|child| {
    info!("Child: RLOC16=0x{:04x}, RSSI={}", child.rloc16, child.last_rssi);
    Ok(())
})?;

// Iterate over neighboring routers (peer FTDs only, not children)
ot.neighbors(|neighbor| {
    info!("Router neighbor: RLOC16=0x{:04x}", neighbor.rloc16);
    Ok(())
})?;

// Configure FTD parameters
ot.set_max_allowed_children(10)?;
ot.set_max_child_ip_addresses(4)?;
```

### Link Mode Configuration

Query and control the Thread link mode (available in both MTD and FTD builds):

```rust
// Query current mode
let mode = ot.get_link_mode();
info!("Device type: {}", if mode.device_type_ftd { "FTD" } else { "MTD" });
info!("RX on when idle: {}", mode.rx_on_when_idle);

// Change to sleepy end device (battery powered)
ot.set_link_mode(LinkModeConfig::sleepy_end_device())?;

// Or construct manually
ot.set_link_mode(LinkModeConfig::new(false, false, false))?;
```

**Note:** OpenThread automatically sets the correct default mode based on
compile-time flags (`ftd` vs `mtd`). Explicit setting is only needed for
advanced use cases (e.g., switching to sleepy mode for battery-powered devices).

**Important:** MTD builds cannot set `device_type_ftd = true` (enforced at runtime).

See [examples/nrf/src/bin/ftd_basic.rs](examples/nrf/src/bin/ftd_basic.rs) for a complete FTD example.
// Configure maximum children
ot.set_max_allowed_children(10)?;

// Monitor connected children
ot.children(|child| {
    info!("Child RLOC16: 0x{:04x}, RSSI: {}", child.rloc16, child.last_rssi);
    Ok(())
})?;

// Monitor neighboring routers
ot.neighbors(|neighbor| {
    info!("Neighbor RLOC16: 0x{:04x}, LQI: {}", neighbor.rloc16, neighbor.link_quality_in);
    Ok(())
})?;

// Get specific router information
let router_info = ot.get_router_info(router_id)?;
```

### Memory Requirements

- **MTD**: ~30-50 KB RAM
- **FTD**: ~80-150 KB RAM (depending on configuration)

For devices with limited RAM, reduce the maximum number of children:

```rust
ot.set_max_allowed_children(10)?; // Default is usually 32
```

### Building FTD Libraries

Pre-built FTD libraries are provided for nRF targets (thumbv7em-none-eabi, thumbv6m-none-eabi).

If you need to rebuild them or build for other targets, see [BUILD_FTD_LIBS.md](BUILD_FTD_LIBS.md).

## Testing

Build and flash the [OT-CLI](https://github.com/espressif/esp-idf/tree/master/examples/openthread/ot_cli) on ESP32C6 or ESP32H2.

```
> dataset set active 0e080000000000010000000300000b35060004001fffe002083a90e3a319a904940708fd1fa298dbd1e3290510fe0458f7db96354eaa6041b880ea9c0f030f4f70656e5468726561642d35386431010258d10410888f813c61972446ab616ee3c556a5910c0402a0f7f8

> dataset active
Active Timestamp: 1
Channel: 11
Channel Mask: 0x07fff800
Ext PAN ID: 3a90e3a319a90494
Mesh Local Prefix: fd1f:a298:dbd1:e329::/64
Network Key: fe0458f7db96354eaa6041b880ea9c0f
Network Name: OpenThread-58d1
PAN ID: 0x58d1
PSKc: 888f813c61972446ab616ee3c556a591
Security Policy: 672 onrc
Done

> ifconfig up

> thread start

```

Flash the example to an ESP32-C6 or ESP32-H2 - use a feature for choosing the build-target.

It should output something like
```
Initializing
Currently assigned addresses
fdde:ad00:beef:0:f9ee:5d6d:7fe6:daab
fe80::6cec:6ace:f5ff:30bc

ChangedFlags(Ipv6AddressAdded | ThreadRoleChanged | ThreadLlAddressChanged | ThreadMeshLocalAddressChanged | ThreadKeySequenceChanged | ThreadNetworkDataChanged | Ipv6MulticastSubscribed | ThreadPanIdChanged | ThreadNetworkNameChanged | ThreadExtendedPanIdChanged | ThreadNetworkKeyChanged | ThreadNetworkInterfaceStateChanged | ActiveDatasetChanged)
Currently assigned addresses
fdde:ad00:beef:0:f9ee:5d6d:7fe6:daab
fe80::6cec:6ace:f5ff:30bc

ChangedFlags(ThreadRoleChanged | ThreadRlocAdded | ThreadPartitionIdChanged | ThreadNetworkDataChanged | PendingDatasetChanged)
Currently assigned addresses
fdde:ad00:beef::ff:fe00:8019
fdde:ad00:beef:0:f9ee:5d6d:7fe6:daab
fe80::6cec:6ace:f5ff:30bc
```

Please note the link-local address. (fe80::...)

Back in the OT-CLI ping the device (using the address from above)
```
> ping fe80::6cec:6ace:f5ff:30bc

16 bytes from fe80:0:0:0:906d:1cce:1bc9:8d07: icmp_seq=15 hlim=64 time=13ms
1 packets transmitted, 1 packets received. Packet loss = 0.0%. Round-trip min/avg/max = 13/13.0/13 ms.
Done
```

Now send and receive a UDP message (using the address from above)
```
> udp open

Done
> udp bind :: 1212

Done
> udp send fe80::6cec:6ace:f5ff:30bc 1212 Hello

Done
5 bytes from fe80::6cec:6ace:f5ff:30bc 1212 Hello
```

So it connected and you can successfully ping the device. Receiving and sending UDP packets also works. 🎉
