//! The main entrypoint for the proxy.

#![deny(rust_2018_idioms, clippy::disallowed_methods, clippy::disallowed_types)]
#![forbid(unsafe_code)]
#![recursion_limit = "256"]

// Emit a compile-time error if no TLS implementations are enabled. When adding
// new implementations, add their feature flags here!
#[cfg(not(any(feature = "meshtls-boring", feature = "meshtls-rustls")))]
compile_error!(
    "at least one of the following TLS implementations must be enabled: 'meshtls-boring', 'meshtls-rustls'"
);

use linkerd_app::{
    core::{metrics::prom, transport::BindTcp},
    trace, Config,
};
use linkerd_signal as signal;
use tokio::{sync::mpsc, time};
pub use tracing::{debug, error, info, warn};

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

mod rt;

const EX_USAGE: i32 = 64;

const VERSION: &str = env!("LINKERD2_PROXY_VERSION");
const DATE: &str = env!("LINKERD2_PROXY_BUILD_DATE");
const VENDOR: &str = env!("LINKERD2_PROXY_VENDOR");
const GIT_SHA: &str = env!("GIT_SHA");
const PROFILE: &str = env!("PROFILE");

fn main() {
    PROXY_BUILD_INFO.set(1.0);
    prom::register_uptime_collector().expect("uptime collector must be valid");

    let trace = match trace::Settings::from_env(time::Instant::now()).init() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Invalid logging configuration: {}", e);
            std::process::exit(EX_USAGE);
        }
    };

    info!("{PROFILE} {VERSION} ({GIT_SHA}) by {VENDOR} on {DATE}",);

    // Load configuration from the environment without binding ports.
    let config = match Config::try_from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Invalid configuration: {}", e);
            std::process::exit(EX_USAGE);
        }
    };

    // Builds a runtime with the appropriate number of cores:
    // `LINKERD2_PROXY_CORES` env or the number of available CPUs (as provided
    // by cgroups, when possible).
    rt::build().block_on(async move {
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
        let shutdown_grace_period = config.shutdown_grace_period;

        let bind = BindTcp::with_orig_dst();
        let app = match config
            .build(bind, bind, BindTcp::default(), shutdown_tx, trace)
            .await
        {
            Ok(app) => app,
            Err(e) => {
                eprintln!("Initialization failure: {}", e);
                std::process::exit(1);
            }
        };

        info!("Admin interface on {}", app.admin_addr());
        info!("Inbound interface on {}", app.inbound_addr());
        info!("Outbound interface on {}", app.outbound_addr());

        match app.tap_addr() {
            None => info!("Tap DISABLED"),
            Some(addr) => info!("Tap interface on {}", addr),
        }

        // TODO distinguish ServerName and Identity.
        info!("Local identity is {}", app.local_server_name());
        let addr = app.identity_addr();
        match addr.identity.value() {
            None => info!("Identity verified via {}", addr.addr),
            Some(tls) => {
                info!("Identity verified via {} ({})", addr.addr, tls.server_id);
            }
        }

        let dst_addr = app.dst_addr();
        match dst_addr.identity.value() {
            None => info!("Destinations resolved via {}", dst_addr.addr),
            Some(tls) => info!(
                "Destinations resolved via {} ({})",
                dst_addr.addr, tls.server_id
            ),
        }

        if let Some(oc) = app.opencensus_addr() {
            match oc.identity.value() {
                None => info!("OpenCensus tracing collector at {}", oc.addr),
                Some(tls) => {
                    info!(
                        "OpenCensus tracing collector at {} ({})",
                        oc.addr, tls.server_id
                    )
                }
            }
        }

        let drain = app.spawn();
        tokio::select! {
            _ = signal::shutdown() => {
                info!("Received shutdown signal");
            }
            _ = shutdown_rx.recv() => {
                info!("Received shutdown via admin interface");
            }
        }
        match time::timeout(shutdown_grace_period, drain.drain()).await {
            Ok(()) => debug!("Shutdown completed gracefully"),
            Err(_) => warn!(
                "Graceful shutdown did not complete in {shutdown_grace_period:?}, terminating now"
            ),
        }
    });
}

lazy_static::lazy_static! {
    static ref PROXY_BUILD_INFO: prom::Gauge = prom::register_gauge!(prom::opts!(
        "proxy_build_info",
        "Proxy build info",
        prom::labels! {
            "version" => VERSION,
            "git_sha" => GIT_SHA,
            "profile" => PROFILE,
            "date" => DATE,
            "vendor" => VENDOR,
        }
    ))
    .unwrap();
}
