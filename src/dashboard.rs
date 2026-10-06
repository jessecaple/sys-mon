//! The single native overview window.

use std::cell::{Cell as LocalCell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;

use crate::presentation::{Cell, DisplayLine, present};
use crate::telemetry::{Reading, Snapshot, TelemetrySession, UnavailableReason};

const STYLE: &str = r#"
.compact-titlebar { min-height: 30px; padding: 0 4px; border-bottom: 1px solid alpha(@window_fg_color, 0.13); }
.compact-titlebar .title { color: alpha(@window_fg_color, 0.68); font-size: 0.9em; font-weight: 600; }
.compact-titlebar windowcontrols { margin: 0; padding: 0; }
.compact-titlebar windowcontrols button { min-width: 24px; min-height: 24px; margin: 0; padding: 0; }
.dashboard { padding: 4px 8px 8px; }
.metric-cell { min-height: 2.8em; padding-top: 0.2em; border-bottom: 1px solid alpha(@window_fg_color, 0.13); }
.metric-left { border-right: 1px solid alpha(@window_fg_color, 0.13); }
.metric-first-row { min-height: 2.2em; padding-top: 0; }
.metric-last-row { border-bottom: none; }
.metric-last-row.metric-left { border-right: none; }
.metric-glyph { color: @window_fg_color; font-size: 1.15em; min-width: 1.2em; }
.metric-name { color: alpha(@window_fg_color, 0.75); font-size: 0.85em; font-weight: 600; min-width: 7em; }
.metric-value { color: @window_fg_color; font-size: 1.45em; font-weight: 600; font-feature-settings: "tnum"; }
.metric-missing { color: alpha(@window_fg_color, 0.62); font-size: 1em; font-weight: 500; }
.metric-rail { margin-top: 2px; min-height: 3px; }
.metric-rail trough { min-height: 3px; border-radius: 0; background: alpha(@window_fg_color, 0.18); }
.metric-rail progress { min-height: 3px; border-radius: 0; background: @window_fg_color; }
.metric-empty-rail { margin-top: 2px; min-height: 3px; background: alpha(@window_fg_color, 0.18); }
"#;

fn glyph(name: &str) -> &'static str {
    match name {
        "cpu" => "▣",
        "temperature" => "♨",
        "ram" => "▤",
        "swap" => "⇄",
        "gpu" => "◉",
        "vram" => "▦",
        "storage" => "◫",
        "network-down" => "↓",
        "network-up" => "↑",
        "storage-read" => "↘",
        "storage-write" => "↗",
        _ => "•",
    }
}

struct CellWidget {
    body: gtk::Box,
    last: RefCell<Option<Cell>>,
    show_empty_rail: bool,
}

impl CellWidget {
    fn new(index: usize) -> Self {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
        body.add_css_class("metric-cell");
        if index.is_multiple_of(2) {
            body.add_css_class("metric-left");
        }
        if index < 2 {
            body.add_css_class("metric-first-row");
        }
        if index >= 10 {
            body.add_css_class("metric-last-row");
        }
        body.set_hexpand(true);
        Self {
            body,
            last: RefCell::new(None),
            show_empty_rail: index == 8 || index == 9,
        }
    }

    fn update(&self, cell: &Cell) {
        if self.last.borrow().as_ref() == Some(cell) {
            return;
        }
        while let Some(child) = self.body.first_child() {
            self.body.remove(&child);
        }
        let grid = gtk::Grid::new();
        grid.set_column_spacing(5);
        grid.set_row_spacing(2);
        grid.set_hexpand(true);
        grid.set_margin_start(8);
        grid.set_margin_end(8);

        let icon = gtk::Label::new(Some(glyph(cell.glyph)));
        icon.add_css_class("metric-glyph");
        icon.set_accessible_role(gtk::AccessibleRole::Presentation);
        icon.set_xalign(0.0);
        grid.attach(&icon, 0, 0, 1, 1);

        let name = gtk::Label::new(Some(cell.label));
        name.add_css_class("metric-name");
        name.set_xalign(0.0);
        grid.attach(&name, 1, 0, 1, 1);

        let mut rails = Vec::new();
        for (index, line) in cell.lines.iter().enumerate() {
            let value = value_label(line);
            grid.attach(&value, 2, index as i32, 1, 1);
            if let Some(fraction) = line.rail {
                let rail = gtk::ProgressBar::new();
                rail.set_fraction(fraction);
                rail.add_css_class("metric-rail");
                rail.update_property(&[
                    gtk::accessible::Property::Label(&line.description),
                    gtk::accessible::Property::ValueMin(0.0),
                    gtk::accessible::Property::ValueMax(100.0),
                    gtk::accessible::Property::ValueNow(fraction * 100.0),
                ]);
                rails.push(rail);
            }
        }
        let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        self.body.append(&spacer);
        let show_empty_rail = self.show_empty_rail && rails.is_empty();
        if rails.is_empty() && !show_empty_rail {
            grid.set_margin_bottom(4);
        }
        self.body.append(&grid);
        for rail in rails {
            self.body.append(&rail);
        }
        if show_empty_rail {
            let rail = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            rail.add_css_class("metric-empty-rail");
            rail.set_accessible_role(gtk::AccessibleRole::Presentation);
            self.body.append(&rail);
        }
        *self.last.borrow_mut() = Some(cell.clone());
    }
}

fn value_label(line: &DisplayLine) -> gtk::Label {
    let value = gtk::Label::new(Some(&line.value));
    value.set_xalign(0.0);
    value.set_hexpand(true);
    value.add_css_class("metric-value");
    if line.rail.is_none()
        && matches!(
            line.value.as_str(),
            "Waiting"
                | "Unavailable"
                | "No access"
                | "Invalid reading"
                | "Source error"
                | "Timed out"
        )
    {
        value.add_css_class("metric-missing");
    }
    value.update_property(&[gtk::accessible::Property::Label(&line.description)]);
    value
}

fn install_style() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(STYLE);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_window(app: &adw::Application, fixture: Option<&Snapshot>) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }
    install_style();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("sys-mon")
        .default_width(520)
        .default_height(290)
        .build();
    window.set_size_request(520, 290);

    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header_handle = gtk::WindowHandle::new();
    let header = gtk::CenterBox::new();
    header.add_css_class("compact-titlebar");
    let title = gtk::Label::new(Some("sys-mon"));
    title.add_css_class("title");
    header.set_center_widget(Some(&title));
    header.set_start_widget(Some(&gtk::WindowControls::new(gtk::PackType::Start)));
    header.set_end_widget(Some(&gtk::WindowControls::new(gtk::PackType::End)));
    header_handle.set_child(Some(&header));
    layout.append(&header_handle);

    let grid = gtk::Grid::new();
    grid.set_column_homogeneous(true);
    grid.add_css_class("dashboard");
    grid.set_hexpand(true);
    grid.set_vexpand(false);
    let widgets: Rc<Vec<CellWidget>> = Rc::new((0..12).map(CellWidget::new).collect());
    for (index, widget) in widgets.iter().enumerate() {
        grid.attach(&widget.body, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
    layout.append(&grid);
    window.set_content(Some(&layout));

    let session = if fixture.is_some() {
        None
    } else {
        match TelemetrySession::start_system() {
            Ok(session) => Some(session),
            Err(error) => {
                eprintln!("telemetry could not start: {error}");
                None
            }
        }
    };
    let session = Rc::new(RefCell::new(session));
    let stopping = Rc::new(LocalCell::new(false));

    let initial_snapshot = fixture.cloned().unwrap_or_else(|| {
        if session.borrow().is_some() {
            Snapshot::default()
        } else {
            startup_failed_snapshot()
        }
    });
    let initial = present(&initial_snapshot);
    for (widget, cell) in widgets.iter().zip(initial.iter()) {
        widget.update(cell);
    }
    let session_for_tick = Rc::clone(&session);
    let widgets_for_tick = Rc::clone(&widgets);
    let stopping_for_tick = Rc::clone(&stopping);
    if fixture.is_none() {
        glib::timeout_add_local(Duration::from_secs(1), move || {
            if stopping_for_tick.get() {
                return glib::ControlFlow::Break;
            }
            if let Some(session) = session_for_tick.borrow().as_ref() {
                let cells = present(&session.snapshot());
                for (widget, cell) in widgets_for_tick.iter().zip(cells.iter()) {
                    widget.update(cell);
                }
            }
            glib::ControlFlow::Continue
        });
    }

    let app_for_close = app.clone();
    let session_for_close = Rc::clone(&session);
    window.connect_close_request(move |window| {
        if stopping.replace(true) {
            return glib::Propagation::Stop;
        }
        window.set_visible(false);
        let done_rx = stop_in_background(session_for_close.borrow_mut().take());
        let app = app_for_close.clone();
        glib::timeout_add_local(Duration::from_millis(40), move || {
            match done_rx.try_recv() {
                Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    app.quit();
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            }
        });
        glib::Propagation::Stop
    });
    window.present();
}

fn stop_in_background(session: Option<TelemetrySession>) -> std::sync::mpsc::Receiver<()> {
    let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
    if let Some(mut session) = session {
        std::thread::spawn(move || {
            session.stop();
            let _ = done_tx.send(());
        });
    } else {
        let _ = done_tx.send(());
    }
    done_rx
}

fn startup_failed_snapshot() -> Snapshot {
    fn failed<T>() -> Reading<T> {
        Reading::Unavailable(UnavailableReason::SourceError)
    }
    let mut snapshot = Snapshot::default();
    snapshot.cpu.usage_percent = failed();
    snapshot.cpu.temperature = failed();
    snapshot.ram = failed();
    snapshot.swap = failed();
    snapshot.gpu.usage_percent = failed();
    snapshot.gpu.vram = failed();
    snapshot.gpu.temperature = failed();
    snapshot.network.down_bytes_per_second = failed();
    snapshot.network.up_bytes_per_second = failed();
    snapshot.storage.read_bytes_per_second = failed();
    snapshot.storage.write_bytes_per_second = failed();
    snapshot.storage.primary_volumes = failed();
    snapshot
}

pub fn run() {
    run_with_snapshot(None);
}

fn run_with_snapshot(snapshot: Option<Snapshot>) {
    let app = adw::Application::builder()
        .application_id("io.github.sys_mon.SysMon")
        .build();
    app.connect_activate(move |app| build_window(app, snapshot.as_ref()));
    app.run_with_args(&["sys-mon"]);
}

#[cfg(debug_assertions)]
pub fn run_fixture(case: &str) -> Result<(), &'static str> {
    use crate::telemetry::{MemoryAmount, PrimaryVolume};

    let mut snapshot = Snapshot::default();
    match case {
        "unavailable" => {
            snapshot.cpu.usage_percent = Reading::Unavailable(UnavailableReason::SourceError);
            snapshot.cpu.temperature = Reading::Unavailable(UnavailableReason::Missing);
            snapshot.ram = Reading::Value(MemoryAmount {
                used_bytes: 0,
                total_bytes: 0,
            });
            snapshot.swap = Reading::Unavailable(UnavailableReason::PermissionDenied);
            snapshot.gpu.usage_percent = Reading::Value(0.0);
            snapshot.gpu.temperature = Reading::Unavailable(UnavailableReason::TimedOut);
            snapshot.gpu.vram = Reading::Unavailable(UnavailableReason::Missing);
            snapshot.storage.primary_volumes = Reading::Unavailable(UnavailableReason::Missing);
            snapshot.network.down_bytes_per_second = Reading::Value(0.0);
            snapshot.network.up_bytes_per_second =
                Reading::Unavailable(UnavailableReason::WarmingUp);
            snapshot.storage.read_bytes_per_second = Reading::Value(0.0);
            snapshot.storage.write_bytes_per_second =
                Reading::Unavailable(UnavailableReason::InvalidData);
        }
        "distinct-storage" => {
            snapshot.storage.primary_volumes = Reading::Value(vec![
                PrimaryVolume {
                    paths: vec!["/".into()],
                    available_bytes: 312 * 1024 * 1024 * 1024,
                    used_bytes: 688 * 1024 * 1024 * 1024,
                    total_bytes: 1000 * 1024 * 1024 * 1024,
                },
                PrimaryVolume {
                    paths: vec!["/home".into()],
                    available_bytes: 80 * 1024 * 1024 * 1024,
                    used_bytes: 120 * 1024 * 1024 * 1024,
                    total_bytes: 200 * 1024 * 1024 * 1024,
                },
            ]);
        }
        _ => return Err("unknown dashboard fixture"),
    }
    run_with_snapshot(Some(snapshot));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{Patch, Provider, ProviderKind, StopToken};

    struct HeldProvider {
        entered: std::sync::mpsc::SyncSender<()>,
        release: std::sync::mpsc::Receiver<()>,
    }

    impl Provider for HeldProvider {
        fn kind(&self) -> ProviderKind {
            ProviderKind::Temperature
        }

        fn sample(&mut self, _elapsed: Duration, stop: &StopToken) -> Patch {
            self.entered.send(()).unwrap();
            self.release.recv().unwrap();
            while !stop.is_stopped() {
                std::thread::yield_now();
            }
            Patch::Temperature(Reading::Unavailable(UnavailableReason::Missing))
        }
    }

    #[test]
    fn startup_failure_has_no_permanent_waiting_cells() {
        let cells = present(&startup_failed_snapshot());
        for cell in cells {
            for line in cell.lines {
                assert_eq!(line.value, "Source error");
                assert!(line.rail.is_none());
            }
        }
    }

    #[test]
    fn close_stop_returns_to_main_loop_during_in_flight_sample() {
        let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(0);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(0);
        let mut builder = TelemetrySession::builder();
        builder
            .register(HeldProvider {
                entered: entered_tx,
                release: release_rx,
            })
            .unwrap();
        let session = builder.start().unwrap();
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        let done = stop_in_background(Some(session));
        assert!(matches!(
            done.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        release_tx.send(()).unwrap();
        done.recv_timeout(Duration::from_secs(1)).unwrap();
    }

    #[test]
    fn close_after_startup_failure_has_no_worker_to_wait_for() {
        stop_in_background(None)
            .recv_timeout(Duration::from_millis(100))
            .unwrap();
    }
}
