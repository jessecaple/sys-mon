use std::time::Duration;
use sys_mon::telemetry::{TelemetrySession, run_temperature_helper};

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--internal-temperature-probe") {
        if let Err(error) = run_temperature_helper() {
            eprintln!("temperature probe failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    // The native GTK window is delivered in M2. This diagnostic path provides
    // a real process-boundary host check while the telemetry subsystem lands.
    if std::env::args().nth(1).as_deref() == Some("--telemetry-once") {
        let mut session = TelemetrySession::start_system().expect("telemetry session should start");
        std::thread::sleep(Duration::from_millis(1400));
        println!("{:#?}", session.snapshot());
        session.stop();
        return;
    }

    println!("sys-mon telemetry is ready; the desktop overview is under construction");
}
