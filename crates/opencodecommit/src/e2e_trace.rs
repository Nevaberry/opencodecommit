use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;

fn trace_path() -> Option<PathBuf> {
    std::env::var_os("OCC_E2E_RESPONSE_LOG").map(PathBuf::from)
}

pub fn log_response(platform: &str, operation: &str, backend: &str, response: &str) {
    let Some(path) = trace_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };

    let test_case = std::env::var("OCC_E2E_TEST_CASE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unspecified".to_owned());

    let response = response.trim();
    let _ = writeln!(file, "=== AI Response ===");
    let _ = writeln!(file, "platform: {platform}");
    let _ = writeln!(file, "test: {test_case}");
    let _ = writeln!(file, "operation: {operation}");
    let _ = writeln!(file, "backend: {backend}");
    let _ = writeln!(file, "response:");
    let _ = writeln!(file, "{response}");
    let _ = writeln!(file);
}
