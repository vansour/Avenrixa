use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn read_stream(stream: Option<impl Read>) -> String {
    let mut output = String::new();
    if let Some(mut stream) = stream {
        let _ = stream.read_to_string(&mut output);
    }
    output
}

#[test]
fn configured_runtime_database_failure_exits_instead_of_falling_back_to_bootstrap() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let bootstrap_path = temp_dir.path().join("bootstrap.json");
    std::fs::write(
        &bootstrap_path,
        r#"{
  "database_kind": "mysql",
  "database_url": "mysql://user:pass@127.0.0.1:3307/bootstrap_fallback"
}"#,
    )
    .expect("bootstrap config should be written");

    let mut child = Command::new(env!("CARGO_BIN_EXE_avenrixa"))
        .env("JWT_SECRET", "short-secret")
        .env("RUST_LOG", "error")
        .env("SERVER_HOST", "127.0.0.1")
        .env("SERVER_PORT", "0")
        .env("DATABASE_KIND", "mysql")
        .env("DATABASE_URL", "not-a-valid-database-url")
        .env("REDIS_URL", "")
        .env("BOOTSTRAP_CONFIG_PATH", &bootstrap_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("backend binary should start");

    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("child status should be readable") {
            break status;
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let stdout = read_stream(child.stdout.take());
            let stderr = read_stream(child.stderr.take());
            panic!(
                "backend kept running; expected fail-closed instead of bootstrap fallback\nstdout:\n{}\nstderr:\n{}",
                stdout, stderr
            );
        }

        thread::sleep(Duration::from_millis(100));
    };

    let stdout = read_stream(child.stdout.take());
    let stderr = read_stream(child.stderr.take());

    assert!(
        !status.success(),
        "backend unexpectedly started successfully\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(
        !stderr.contains("Server listening on"),
        "backend should not start serving requests after runtime init failure\nstderr:\n{}",
        stderr
    );
    assert!(
        stderr.contains("refusing to expose bootstrap mode")
            || stderr.contains("Runtime initialization failed")
            || stderr.contains("Configuration validation failed")
            || stderr.contains("MySQL / MariaDB 数据库 URL")
            || stderr.contains("JWT_SECRET"),
        "stderr should indicate fail-closed startup behavior\nstderr:\n{}",
        stderr
    );
}
