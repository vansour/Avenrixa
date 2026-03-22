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
            "database_kind": "postgresql",
            "database_url": "not-a-valid-database-url"
        }"#,
    )
    .expect("bootstrap config should be written");

    let mut child = Command::new(env!("CARGO_BIN_EXE_avenrixa"))
        .env("JWT_SECRET", "short-secret")
        .env("RUST_LOG", "error")
        .env("SERVER_HOST", "127.0.0.1")
        .env("SERVER_PORT", "0")
        .env("DATABASE_KIND", "postgresql")
        .env("DATABASE_URL", "not-a-valid-database-url")
        .env("CACHE_URL", "")
        .env("BOOTSTRAP_CONFIG_PATH", &bootstrap_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("backend binary should start");

    let stdout_handle = thread::spawn({
        let stdout = child.stdout.take();
        move || read_stream(stdout)
    });
    let stderr_handle = thread::spawn({
        let stderr = child.stderr.take();
        move || read_stream(stderr)
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("child status should be readable") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            break child
                .wait()
                .expect("child status should be available after kill");
        }
        thread::sleep(Duration::from_millis(50));
    };
    let stdout = stdout_handle
        .join()
        .expect("stdout reader thread should complete");
    let stderr = stderr_handle
        .join()
        .expect("stderr reader thread should complete");

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
    let runtime_fail_closed = stderr.contains("Runtime initialization failed")
        && stderr.contains("refusing to expose bootstrap mode");
    let configuration_validation_failed = stderr.contains("Configuration validation failed")
        || stderr.contains("PostgreSQL 数据库 URL")
        || stderr.contains("JWT_SECRET");

    assert!(
        runtime_fail_closed || configuration_validation_failed,
        "stderr should indicate failed startup behavior\nstderr:\n{}",
        stderr
    );
}
