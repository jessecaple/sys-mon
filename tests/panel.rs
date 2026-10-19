use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[test]
fn panel_stream_has_real_readings_and_stops_when_its_owner_closes_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sys-mon"))
        .arg("--panel-stream")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let result = std::panic::catch_unwind(|| {
        for _ in 0..2 {
            let line = receiver
                .recv_timeout(Duration::from_secs(6))
                .unwrap()
                .unwrap();
            let frame: serde_json::Value = serde_json::from_str(&line).unwrap();
            assert_eq!(frame["version"], 1);
            let cells = frame["cells"].as_array().unwrap();
            assert_eq!(cells.len(), 14);
            assert_eq!(cells[8]["label"], "/tmp");
            assert_eq!(cells[9]["label"], "Inodes");
            assert!(
                cells[9]["lines"][0]["description"]
                    .as_str()
                    .unwrap()
                    .starts_with("/tmp inodes:")
            );
        }
    });
    drop(child.stdin.take());
    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
    assert!(
        status
            .expect("backend must exit after stdin closes")
            .success()
    );
}
