use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns the path to the compiled `ferrite` binary.
fn ferrite_bin() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("ferrite");
    #[cfg(windows)]
    path.set_extension("exe");
    path
}

fn run_test(path: &Path, args: &[&str]) -> (i32, String) {
    let output = Command::new(ferrite_bin())
        .args(args)
        .arg(path)
        .output()
        .expect("Failed to execute ferrite binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    (exit_code, combined)
}

#[test]
fn run_pass_tests() {
    let tests_dir = Path::new("tests");
    let mut entries: Vec<_> = fs::read_dir(tests_dir)
        .unwrap()
        .map(|r| r.unwrap().path())
        .filter(|p| {
            p.is_file()
                && p.extension().map_or(false, |ext| ext == "fe")
                && p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("pass_")
        })
        .collect();

    entries.sort();

    for test_file in entries {
        let test_name = test_file.file_stem().unwrap().to_str().unwrap();

        // 1. Check (Compile)
        let (check_code, check_out) = run_test(&test_file, &["check"]);
        if check_code != 0 {
            panic!(
                "PASS test '{}' failed compilation with code {}:\n{}",
                test_name, check_code, check_out
            );
        }

        // 2. Run
        let (run_code, run_out) = run_test(&test_file, &["run"]);
        if run_code != 0 {
            panic!(
                "PASS test '{}' compiled but failed execution with code {}:\n{}",
                test_name, run_code, run_out
            );
        }

        // Use insta to snapshot the run output (if any) to prevent regressions
        insta::with_settings!({
            snapshot_path => "snapshots/pass",
            prepend_module_to_snapshot => false,
        }, {
            insta::assert_snapshot!(test_name, run_out);
        });
    }
}

#[test]
fn run_fail_tests() {
    let tests_dir = Path::new("tests");
    let mut entries: Vec<_> = fs::read_dir(tests_dir)
        .unwrap()
        .map(|r| r.unwrap().path())
        .filter(|p| {
            p.is_file()
                && p.extension().map_or(false, |ext| ext == "fe")
                && p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("fail_")
        })
        .collect();

    entries.sort();

    for test_file in entries {
        let test_name = test_file.file_stem().unwrap().to_str().unwrap();

        // For fail tests, they MUST fail compilation (check)
        let (check_code, check_out) = run_test(&test_file, &["check"]);
        if check_code == 0 {
            panic!(
                "FAIL test '{}' incorrectly compiled successfully:\n{}",
                test_name, check_out
            );
        }

        // Snapshot the compiler error to ensure error messages don't regress!
        insta::with_settings!({
            snapshot_path => "snapshots/fail",
            prepend_module_to_snapshot => false,
        }, {
            // Strip out absolute file paths so snapshots are portable
            let normalized_out = check_out.replace('\\', "/");
            // Normalize path separators to avoid windows/unix diffs
            let normalized_out = normalized_out.replace("d:/ferrite/tests/", "tests/");
            insta::assert_snapshot!(test_name, normalized_out);
        });
    }
}
