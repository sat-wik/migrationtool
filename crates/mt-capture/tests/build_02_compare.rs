#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-02 (WR-05): the determinism comparison of two capture output trees fails
//! closed on every unreadable, empty, one-sided or missing file, and leaves out
//! exactly the two documented step outputs. Plan 02-06 measured both exclusions.

use std::path::{Path, PathBuf};

use mt_capture::compare::{DETERMINISM_EXCLUSIONS, DirComparison, compare_capture_dirs};
use serde_json::Value;

const REPORT: &str = "corejson/corejson-default.json";
const COMMANDS: &str = "corejson/corejson-default.compile_commands.json";

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ci")
        .join(name);
    std::fs::read_to_string(&path).unwrap()
}

fn report() -> Value {
    serde_json::from_str(&fixture("corejson-default.capture.json")).unwrap()
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn write_json(root: &Path, rel: &str, value: &Value) {
    write(root, rel, &serde_json::to_string_pretty(value).unwrap());
}

/// Two trees holding the committed CI capture report and compile_commands file.
struct Pair {
    _hold: tempfile::TempDir,
    first: PathBuf,
    second: PathBuf,
}

impl Pair {
    fn new() -> Self {
        let hold = tempfile::tempdir().unwrap();
        let (first, second) = (hold.path().join("first"), hold.path().join("second"));
        for root in [&first, &second] {
            write_json(root, REPORT, &report());
            write(
                root,
                COMMANDS,
                &fixture("corejson-default.compile_commands.json"),
            );
        }
        Self {
            _hold: hold,
            first,
            second,
        }
    }

    fn compare(&self) -> DirComparison {
        compare_capture_dirs(&self.first, &self.second).unwrap()
    }
}

type StepMatch = fn(&str) -> bool;
/// A step, the path of the field inside it, and the value it is changed to.
type Case = (&'static str, StepMatch, &'static [&'static str], Value);

fn is_configure(name: &str) -> bool {
    name == "cmake configure"
}

fn is_build(name: &str) -> bool {
    name == "cmake build"
}

fn is_clang_ast(name: &str) -> bool {
    name.starts_with("clang ast ")
}

fn step_mut(report: &mut Value, matches: impl Fn(&str) -> bool) -> &mut Value {
    report["steps"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|step| matches(step["name"].as_str().unwrap()))
        .expect("the fixture has that step")
}

/// Change one field of the second tree's report and return the verdict.
fn compare_after(edit: impl Fn(&mut Value)) -> DirComparison {
    let pair = Pair::new();
    let mut changed = report();
    edit(&mut changed);
    write_json(&pair.second, REPORT, &changed);
    pair.compare()
}

fn mentions(comparison: &DirComparison, needles: &[&str]) -> bool {
    comparison
        .problems
        .iter()
        .any(|problem| needles.iter().all(|needle| problem.contains(needle)))
}

#[test]
fn build_02_compare_identical_trees_pass() {
    let pair = Pair::new();
    let comparison = pair.compare();
    assert_eq!(comparison.compared, 2);
    assert!(comparison.problems.is_empty(), "{:?}", comparison.problems);
    assert!(comparison.passed());
}

#[test]
fn build_02_compare_fails_closed_on_unparseable_reports() {
    // One side unparseable.
    let pair = Pair::new();
    write(&pair.second, REPORT, "{\"kind\": \"capture\", \"steps\": [");
    let comparison = pair.compare();
    assert!(!comparison.passed());
    assert!(
        mentions(&comparison, &["cannot parse second/", REPORT]),
        "{:?}",
        comparison.problems
    );

    // Both sides unparseable (the jq | cmp -s case): each side is named.
    let pair = Pair::new();
    for root in [&pair.first, &pair.second] {
        write(root, REPORT, "not json at all");
    }
    let comparison = pair.compare();
    assert!(!comparison.passed());
    assert!(
        mentions(&comparison, &["cannot parse first/", REPORT]),
        "{:?}",
        comparison.problems
    );
    assert!(
        mentions(&comparison, &["cannot parse second/", REPORT]),
        "{:?}",
        comparison.problems
    );
}

#[test]
fn build_02_compare_flags_empty_and_one_sided_files() {
    // A zero-byte file on either side.
    for empty_side in ["first", "second"] {
        let pair = Pair::new();
        let root = if empty_side == "first" {
            &pair.first
        } else {
            &pair.second
        };
        write(root, REPORT, "");
        let comparison = pair.compare();
        assert!(!comparison.passed(), "{empty_side}");
        assert!(
            mentions(&comparison, &["empty", empty_side, REPORT]),
            "{empty_side}: {:?}",
            comparison.problems
        );
    }

    // A zero-byte compile_commands file on both sides is still empty, not equal.
    let pair = Pair::new();
    for root in [&pair.first, &pair.second] {
        write(root, COMMANDS, "");
    }
    let comparison = pair.compare();
    assert!(!comparison.passed());
    assert!(
        mentions(&comparison, &["empty", COMMANDS]),
        "{:?}",
        comparison.problems
    );

    // A file present on one side only.
    for (only_side, extra_side) in [("first", "first"), ("second", "second")] {
        let pair = Pair::new();
        let root = if extra_side == "first" {
            &pair.first
        } else {
            &pair.second
        };
        write(root, "corejson/variant.json", "{}");
        let comparison = pair.compare();
        assert!(!comparison.passed(), "{only_side}");
        assert!(
            mentions(
                &comparison,
                &["only in", only_side, "corejson/variant.json"]
            ),
            "{only_side}: {:?}",
            comparison.problems
        );
    }
}

#[cfg(unix)]
#[test]
fn build_02_compare_flags_non_regular_files() {
    // A symlink is never followed and never compared as if it were its target.
    let pair = Pair::new();
    let target = pair.first.join(REPORT);
    let link = pair.second.join("corejson/link.json");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    std::os::unix::fs::symlink(&target, pair.first.join("corejson/link.json")).unwrap();
    let comparison = pair.compare();
    assert!(!comparison.passed());
    assert!(
        mentions(&comparison, &["not a regular file", "corejson/link.json"]),
        "{:?}",
        comparison.problems
    );
}

#[test]
fn build_02_compare_with_no_files_does_not_pass() {
    let hold = tempfile::tempdir().unwrap();
    let (first, second) = (hold.path().join("a"), hold.path().join("b"));
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    let comparison = compare_capture_dirs(&first, &second).unwrap();
    assert_eq!(comparison.compared, 0);
    assert!(!comparison.passed());

    // An unreadable top directory is an error, not an empty tree.
    assert!(compare_capture_dirs(&first, &hold.path().join("absent")).is_err());
    assert!(compare_capture_dirs(&hold.path().join("absent"), &second).is_err());
}

#[test]
fn build_02_compare_excludes_only_the_documented_step_outputs() {
    assert_eq!(DETERMINISM_EXCLUSIONS.len(), 2);

    // The excluded fields: stdout sha256, bytes_total, bytes_stored of the step
    // `cmake configure` and of a step named `clang ast X`.
    for (label, matches) in [
        ("cmake configure", is_configure as StepMatch),
        ("clang ast X", is_clang_ast),
    ] {
        for field in ["sha256", "bytes_total", "bytes_stored"] {
            let comparison = compare_after(|changed| {
                let step = step_mut(changed, matches);
                step["run"]["stdout"][field] = if field == "sha256" {
                    Value::from("0".repeat(64))
                } else {
                    Value::from(999_999)
                };
            });
            assert!(
                comparison.passed(),
                "{label}.stdout.{field}: {:?}",
                comparison.problems
            );
        }
    }

    // Everything else of those steps, and the same fields of other steps, are compared.
    let cases: [Case; 4] = [
        (
            "cmake build stdout sha256",
            is_build,
            &["run", "stdout", "sha256"],
            Value::from("0".repeat(64)),
        ),
        (
            "cmake build stdout bytes_total",
            is_build,
            &["run", "stdout", "bytes_total"],
            Value::from(999_999),
        ),
        (
            "cmake configure stderr sha256",
            is_configure,
            &["run", "stderr", "sha256"],
            Value::from("0".repeat(64)),
        ),
        (
            "clang ast argv",
            is_clang_ast,
            &["run", "argv"],
            Value::from(vec!["clang", "-different"]),
        ),
    ];
    for (label, matches, path, replacement) in cases {
        let comparison = compare_after(|changed| {
            let mut slot = step_mut(changed, matches);
            for key in path {
                slot = &mut slot[*key];
            }
            *slot = replacement.clone();
        });
        assert!(!comparison.passed(), "{label} must be compared");
        assert!(
            mentions(&comparison, &[REPORT, "/steps/"]),
            "{label}: the problem names the file and the JSON pointer: {:?}",
            comparison.problems
        );
    }

    // The exact JSON pointer of the first difference.
    let comparison = compare_after(|changed| {
        let step = step_mut(changed, |name| name == "cmake build");
        step["run"]["stdout"]["sha256"] = Value::from("0".repeat(64));
    });
    assert!(
        mentions(&comparison, &[REPORT, "/steps/2/run/stdout/sha256"]),
        "{:?}",
        comparison.problems
    );

    // An entry result, an object hash and a configure-file hash (each a map in the
    // report; the first key of each is changed).
    for section in ["entries", "objects", "configure_files"] {
        let key = report()[section]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .expect("the fixture has one")
            .clone();
        let pointer = format!("/{section}/{}", key.replace('~', "~0").replace('/', "~1"));
        let comparison = compare_after(|changed| {
            let slot = changed.pointer_mut(&pointer).expect("the fixture has it");
            *slot = Value::from("changed");
        });
        assert!(!comparison.passed(), "{pointer} must be compared");
        assert!(
            mentions(&comparison, &[REPORT, &pointer]),
            "{pointer}: {:?}",
            comparison.problems
        );
    }
}

#[test]
fn build_02_compare_ignores_key_order_but_not_values() {
    let pair = Pair::new();
    write(
        &pair.first,
        "x/r.json",
        "{\"kind\":\"capture\",\"steps\":[],\"a\":1,\"b\":[1,2]}",
    );
    write(
        &pair.second,
        "x/r.json",
        "{\"b\":[1,2],\"a\":1,\"steps\":[],\"kind\":\"capture\"}",
    );
    let comparison = pair.compare();
    assert!(comparison.passed(), "{:?}", comparison.problems);

    // A changed array order is a difference, at the first differing position.
    write(
        &pair.second,
        "x/r.json",
        "{\"kind\":\"capture\",\"steps\":[],\"a\":1,\"b\":[2,1]}",
    );
    let comparison = pair.compare();
    assert!(!comparison.passed());
    assert!(
        mentions(&comparison, &["x/r.json", "/b/0"]),
        "{:?}",
        comparison.problems
    );

    // A key present on one side only is named by its pointer.
    write(
        &pair.second,
        "x/r.json",
        "{\"kind\":\"capture\",\"steps\":[],\"a\":1,\"b\":[1,2],\"c\":0}",
    );
    let comparison = pair.compare();
    assert!(
        mentions(&comparison, &["x/r.json", "/c"]),
        "{:?}",
        comparison.problems
    );
}

#[test]
fn build_02_compare_compile_commands_byte_for_byte() {
    // A whitespace-only change is a difference for a compile_commands file.
    let pair = Pair::new();
    let text = fixture("corejson-default.compile_commands.json");
    write(&pair.second, COMMANDS, &format!("{text}\n"));
    let comparison = pair.compare();
    assert!(!comparison.passed());
    assert!(
        mentions(&comparison, &["byte for byte", COMMANDS]),
        "{:?}",
        comparison.problems
    );

    // The exclusions never apply to it, even when its content looks like a capture report.
    let pair = Pair::new();
    let mut lookalike = report();
    write_json(&pair.first, COMMANDS, &lookalike);
    step_mut(&mut lookalike, |name| name == "cmake configure")["run"]["stdout"]["bytes_total"] =
        Value::from(999_999);
    write_json(&pair.second, COMMANDS, &lookalike);
    let comparison = pair.compare();
    assert!(!comparison.passed());
    assert!(
        mentions(&comparison, &["byte for byte", COMMANDS]),
        "{:?}",
        comparison.problems
    );
}

#[test]
fn build_02_compare_exclusions_apply_to_capture_reports_only() {
    // The same step digests that pass in a capture report are differences in a
    // replay or variant report.
    for kind in ["replay", "variant"] {
        let comparison = compare_after_kind(kind);
        assert!(!comparison.passed(), "{kind}");
        assert!(
            mentions(&comparison, &[REPORT, "/steps/1/run/stdout/sha256"]),
            "{kind}: {:?}",
            comparison.problems
        );
    }
}

fn compare_after_kind(kind: &str) -> DirComparison {
    let pair = Pair::new();
    let mut first = report();
    first["kind"] = Value::from(kind);
    let mut second = first.clone();
    step_mut(&mut second, |name| name == "cmake configure")["run"]["stdout"]["sha256"] =
        Value::from("0".repeat(64));
    write_json(&pair.first, REPORT, &first);
    write_json(&pair.second, REPORT, &second);
    pair.compare()
}
