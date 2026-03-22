use std::fs;
use std::path::PathBuf;
use src_core::parser::Parser;
use src_core::html::push_html;

fn render_body(md: &str) -> String {
    let parser = Parser::new(md);
    let mut out = String::new();
    push_html(&mut out, parser);
    out
}

fn testcases_dir() -> PathBuf {
    // Cargo sets CARGO_MANIFEST_DIR to the crate root at test time.
    // src-core lives one level inside the workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .join("tests/testcases")
}

/// Collect all *.n.md files from tests/testcases/, parse them,
/// and compare with the matching *.html expectation file.
/// If a .html file doesn't exist yet it is generated and the test passes;
/// subsequent runs will compare against that snapshot.
#[test]
fn snapshot_testcases() {
    let dir = testcases_dir();
    assert!(dir.exists(), "testcases dir not found: {}", dir.display());

    let mut any_failed = false;

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("can't read testcases dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".n.md"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let md_path = entry.path();
        let html_path = md_path.with_extension("").with_extension("html"); // strips .n.md → .html

        let md = fs::read_to_string(&md_path)
            .expect(&format!("can't read {}", md_path.display()));

        let actual = render_body(&md);

        if !html_path.exists() {
            // First run – write the snapshot.
            fs::write(&html_path, &actual)
                .expect(&format!("can't write snapshot {}", html_path.display()));
            println!("[SNAPSHOT CREATED] {}", html_path.file_name().unwrap().to_string_lossy());
            continue;
        }

        let expected = fs::read_to_string(&html_path)
            .expect(&format!("can't read {}", html_path.display()));

        if actual == expected {
            println!("[PASS] {}", md_path.file_name().unwrap().to_string_lossy());
        } else {
            eprintln!("[FAIL] {}", md_path.file_name().unwrap().to_string_lossy());
            // Show a simple diff (first differing line)
            let act_lines: Vec<&str> = actual.lines().collect();
            let exp_lines: Vec<&str> = expected.lines().collect();
            let max = act_lines.len().max(exp_lines.len());
            for i in 0..max {
                let a = act_lines.get(i).copied().unwrap_or("<missing>");
                let e = exp_lines.get(i).copied().unwrap_or("<missing>");
                if a != e {
                    eprintln!("  line {} actual  : {:?}", i + 1, a);
                    eprintln!("  line {} expected : {:?}", i + 1, e);
                    break;
                }
            }
            any_failed = true;
        }
    }

    assert!(!any_failed, "One or more snapshot tests failed (see above)");
}
