//! FTD example for NRF, demonstrating Full Thread Device functionality.
//!
//! This example creates an FTD device that can act as a Router or Leader in the Thread network,
//! accept child devices, and relay traffic. It demonstrates FTD-specific APIs like child management
//! and neighbor tracking.
//!
//! Hardware: nRF52840 or similar with IEEE 802.15.4 radio
//!
//! This device will:
//! - Form or join a Thread network as an FTD
//! - Accept child devices (MTD or FTD)
//! - Monitor and log connected children
//! - Provide UDP echo service on port 1212

#![no_std]
#![no_main]

use core::net::{Ipv6Addr, SocketAddrV6};

use defmt::info;

use embassy_executor::InterruptExecutor;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use embassy_nrf::interrupt;
use embassy_nrf::interrupt::{InterruptExt, Priority};
use embassy_nrf::mode::Blocking;
use embassy_nrf::rng::Rng;
use embassy_nrf::{bind_interrupts, peripherals, radio};

use openthread::nrf::{Ieee802154, NrfRadio};
use openthread::{
    BytesFmt, DeviceRole, EmbassyTimeTimer, OpenThread, OtResources, OtUdpResources,
    PhyRadioRunner, ProxyRadio, ProxyRadioResources, SimpleRamSettings, UdpSocket,
};

use panic_rtt_target as _;

use rand_core::RngCore;

use tinyrlibc as _;

macro_rules! mk_static {
    ($t:ty) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit();
        x
    }};
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write($val);
        x
    }};
}

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
});

#[interrupt]
unsafe fn EGU0_SWI0() {
    EXECUTOR_HIGH.on_interrupt()
}

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

const BOUND_PORT: u16 = 1212;

const UDP_SOCKETS_BUF: usize = 1280;
const UDP_MAX_SOCKETS: usize = 2;

// Default Thread network dataset
const THREAD_DATASET: &str = if let Some(dataset) = option_env!("THREAD_DATASET") {
    dataset
} else {
    "0e080000000000010000000300000b35060004001fffe002083a90e3a319a904940708fd1fa298dbd1e3290510fe0458f7db96354eaa6041b880ea9c0f030f4f70656e5468726561642d35386431010258d10410888f813c61972446ab616ee3c556a5910c0402a0f7f8"
};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;

    let p = embassy_nrf::init(config);

    rtt_target::rtt_init_defmt!();

    info!("Starting OpenThread FTD example...");

    let rng = mk_static!(Rng<'static, Blocking>, Rng::new_blocking(p.RNG));

    let mut ieee_eui64 = [0; 8];
    RngCore::fill_bytes(rng, &mut ieee_eui64);

    let ot_resources = mk_static!(OtResources, OtResources::new());
    let ot_udp_resources =
        mk_static!(OtUdpResources<UDP_MAX_SOCKETS, UDP_SOCKETS_BUF>, OtUdpResources::new());
    let ot_settings_buf = mk_static!([u8; 1024], [0; 1024]);

    let ot_settings = mk_static!(SimpleRamSettings, SimpleRamSettings::new(ot_settings_buf));

    let ot = OpenThread::new_with_udp(ieee_eui64, rng, ot_settings, ot_resources, ot_udp_resources)
        .unwrap();

    info!("OpenThread instance created");

    // Configure FTD-specific parameters
    #[cfg(feature = "ftd")]
    {
        // Set maximum children to a reasonable value for nRF52840 (has ~256KB RAM)
        // Default is 32, but we'll use 10 to be conservative
        ot.set_max_allowed_children(10).unwrap();
        info!("Max children set to 10");

        // Allow multiple IP addresses per child
        ot.set_max_child_ip_addresses(4).unwrap();
        info!("Max IP addresses per child set to 4");
    }

    let mut radio = NrfRadio::new(Ieee802154::new(p.RADIO, Irqs));

    let proxy_radio_resources = mk_static!(ProxyRadioResources, ProxyRadioResources::new());
    let (proxy_radio, phy_radio_runner) = ProxyRadio::new(radio.caps(), proxy_radio_resources);

    // High-priority executor for radio operations
    interrupt::EGU0_SWI0.set_priority(Priority::P7);

    let spawner_high = EXECUTOR_HIGH.start(interrupt::EGU0_SWI0);
    spawner_high
        .spawn(run_radio(phy_radio_runner, radio))
        .unwrap();

    info!("Radio initialized");

    spawner.spawn(run_ot(ot.clone(), proxy_radio)).unwrap();
    spawner.spawn(run_ot_status_monitor(ot.clone())).unwrap();

    #[cfg(feature = "ftd")]
    spawner.spawn(run_ftd_monitor(ot.clone())).unwrap();

    info!("Configuring Thread network...");
    info!("Dataset: {}", THREAD_DATASET);

    ot.set_active_dataset_tlv_hexstr(THREAD_DATASET).unwrap();
    ot.enable_ipv6(true).unwrap();
    ot.enable_thread(true).unwrap();

    let socket = UdpSocket::bind(
        ot,
        &SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, BOUND_PORT, 0, 0),
    )
    .unwrap();

    info!("UDP socket bound to port {}", BOUND_PORT);
    info!("FTD device ready - waiting for network formation and child connections...");

    let buf: &mut [u8] = unsafe { mk_static!([u8; UDP_SOCKETS_BUF]).assume_init_mut() };

    loop {
        let (len, local, remote) = socket.recv(buf).await.unwrap();

        info!("UDP: Received {} from {} on {}", BytesFmt(&buf[..len]), remote, local);

        socket.send(b"Hello from FTD", Some(&local), &remote).await.unwrap();
        info!("UDP: Sent response");
    }
}

#[embassy_executor::task]
async fn run_radio<R>(mut runner: PhyRadioRunner<'static, R>, radio: R) -> !
where
    R: openthread::Radio,
{
    runner.run(radio, EmbassyTimeTimer).await
}

#[embassy_executor::task]
async fn run_ot<R>(ot: OpenThread<'static>, radio: R) -> !
where
    R: openthread::Radio,
{
    ot.run(radio).await
}

#[embassy_executor::task]
async fn run_ot_status_monitor(ot: OpenThread<'static>) {
    let mut last_role = DeviceRole::Disabled;

    loop {
        ot.wait_changed().await;

        let status = ot.net_status();

        if status.role != last_role {
            info!("Role changed: {:?} -> {:?}", last_role, status.role);
            last_role = status.role;

            match status.role {
                DeviceRole::Leader => {
                    info!("*** DEVICE IS NOW LEADER ***");
                    info!("This device formed the network and can accept children");
                }
                DeviceRole::Router => {
                    info!("*** DEVICE IS NOW ROUTER ***");
                    info!("This device joined as router and can accept children");
                }
                DeviceRole::Child => {
                    info!("Device is a child (should not happen in FTD mode)");
                }
                DeviceRole::Detached => {
                    info!("Device detached from network");
                }
                _ => {}
            }
        }

        if status.role.is_connected() {
            let mut count = 0;
            let _ = ot.ipv6_addrs(|addr| {
                if let Some((ip, prefix)) = addr {
                    if count == 0 {
                        info!("IPv6 addresses:");
                    }
                    info!("  {}/{}", ip, prefix);
                    count += 1;
                }
                Ok(())
            });
        }
    }
}

/// FTD-specific monitoring task that tracks children and neighbors
#[cfg(feature = "ftd")]
#[embassy_executor::task]
async fn run_ftd_monitor(ot: OpenThread<'static>) {
    info!("FTD monitor started");

    loop {
        Timer::after(Duration::from_secs(30)).await;

        let status = ot.net_status();

        // Only monitor when we're a router or leader
        if status.role == DeviceRole::Router || status.role == DeviceRole::Leader {
            info!("=== FTD Status Report ===");
            info!("Role: {:?}", status.role);

            // Count and display children
            let mut child_count = 0;
            let _ = ot.children(|child| {
                if child_count == 0 {
                    info!("Connected children:");
                }
                info!(
                    "  Child {}: RLOC16=0x{:04x}, Age={}s, RSSI={}, RxOnIdle={}",
                    child_count, child.rloc16, child.age, child.last_rssi, child.rx_on_when_idle
                );
                child_count += 1;
                Ok(())
            });

            if child_count == 0 {
                info!("No children connected (max: {})", ot.get_max_allowed_children());
            } else {
                info!("Total children: {}/{}", child_count, ot.get_max_allowed_children());
            }

            // Count and display neighbors (other routers)
            let mut neighbor_count = 0;
            let _ = ot.neighbors(|neighbor| {
                if neighbor_count == 0 {
                    info!("Neighboring routers:");
                }
                info!(
                    "  Router {}: RLOC16=0x{:04x}, LQI={}, RSSI={}",
                    neighbor_count, neighbor.rloc16, neighbor.link_quality_in, neighbor.last_rssi
                );
                neighbor_count += 1;
                Ok(())
            });

            if neighbor_count > 0 {
                info!("Total neighbors: {}", neighbor_count);
            }

            info!("========================");
        }
    }
}
