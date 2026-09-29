//! rgbeastd: the RGBeast lighting daemon. The only process that opens hidraw and
//! i2c device nodes. It exposes a small, typed D-Bus API guarded by polkit,
//! restores lighting at boot and after sleep, and can run against simulated
//! devices for development.

mod auth;
mod hw;
mod service;
mod store;

use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::EnvFilter;

pub const BUS_NAME: &str = "io.github.djshiye.RGBeast1";
pub const OBJECT_PATH: &str = "/io/github/djshiye/RGBeast1";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(name = "rgbeastd", version, about = "RGBeast lighting daemon")]
struct Args {
    /// Register on the session bus instead of the system bus (development).
    #[arg(long)]
    session: bool,
    /// Use simulated devices instead of real hardware.
    #[arg(long)]
    simulate: bool,
    /// Print the devices that would be detected, then exit.
    #[arg(long)]
    scan: bool,
    /// Where to keep state.json (default: $STATE_DIRECTORY or /var/lib/rgbeast).
    #[arg(long)]
    state_dir: Option<PathBuf>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .without_time()
        .init();
    let args = Args::parse();

    let state_dir = args
        .state_dir
        .or_else(|| std::env::var_os("STATE_DIRECTORY").map(PathBuf::from))
        .unwrap_or_else(rgbeast_core::discover::default_state_dir);
    let store = store::Store::open(&state_dir);

    if args.scan {
        let found = hw::scan(args.simulate, &store.discovery_config());
        for line in &found.log {
            println!("{line}");
        }
        println!("{} device(s):", found.devices.len());
        for d in &found.devices {
            let i = d.info();
            println!(
                "  {} [{}] {} — {} zones, {} LEDs ({})",
                i.id,
                i.driver,
                i.name,
                i.zones.len(),
                i.led_count(),
                i.location
            );
        }
        return Ok(());
    }

    let worker = hw::Worker::spawn(args.simulate, store.clone());
    let manager =
        service::Manager::new(worker.clone(), store.clone(), args.simulate, !args.session);

    let builder = if args.session {
        zbus::connection::Builder::session()?
    } else {
        zbus::connection::Builder::system()?
    };
    let conn = builder
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, manager)?
        .build()
        .await?;
    tracing::info!(
        bus = if args.session { "session" } else { "system" },
        "rgbeastd {} ready",
        VERSION
    );

    if !args.session {
        let resume_conn = conn.clone();
        let resume_worker = worker.clone();
        let resume_store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = service::watch_sleep(resume_conn, resume_worker, resume_store).await {
                tracing::warn!("sleep monitoring unavailable: {e}");
            }
        });
    }

    // At boot (or right after installation) the daemon can start before udev
    // has applied the rules or before amdgpu has created the GPU's bus. If
    // nothing was found, a device this machine had last time is missing, or a
    // node could not be opened, look again after 10 s and once more at 30 s.
    if !args.simulate {
        let known = store.known_devices();
        let retry_worker = worker.clone();
        tokio::spawn(async move {
            for delay in [10u64, 20] {
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                let Ok(found) = retry_worker.list().await else {
                    return;
                };
                let log = retry_worker.log().await.unwrap_or_default();
                let denied = log.iter().any(|l| l.contains("Permission denied"));
                let missing: Vec<&String> = known
                    .iter()
                    .filter(|id| !found.iter().any(|d| &d.id == *id))
                    .collect();
                if found.is_empty() || denied || !missing.is_empty() {
                    tracing::info!(
                        "scanning again ({} found, {} missing, permission problems: {denied})",
                        found.len(),
                        missing.len()
                    );
                    retry_worker.rescan().await.ok();
                } else {
                    return;
                }
            }
        });
    }

    // Forward hardware-side events (rescans, restores) to D-Bus signals.
    let mut events = worker.subscribe();
    let signal_conn = conn.clone();
    tokio::spawn(async move {
        while let Ok(ev) = events.recv().await {
            service::emit(&signal_conn, ev).await;
        }
    });

    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
    tracing::info!("shutting down");
    Ok(())
}
