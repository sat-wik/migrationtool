#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: static checks of the `run_vectors` helper and the self-test step of
//! `.github/workflows/testbed.yml`. They read the repository file and run under
//! plain `cargo test`, with no Docker and no network.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

const WORKFLOW: &str = ".github/workflows/testbed.yml";
const SELFTEST_STEP: &str = "Run the executor self-test in the pinned image";

/// The eight `testbed-` titles of Phase 2; no others may be published.
const PHASE_2_TITLES: [&str; 8] = [
    "testbed-image",
    "testbed-validate",
    "testbed-fetch",
    "testbed-seeds",
    "testbed-build",
    "testbed-capture",
    "testbed-replay",
    "testbed-determinism",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn workflow() -> String {
    let path = repo_root().join(WORKFLOW);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
}

/// The lines of the shell function `name` (from `name() {` to its closing `}`).
fn function_body(text: &str, name: &str) -> String {
    let header = format!("{name}() {{");
    let mut lines = text.lines().skip_while(|line| line.trim() != header);
    assert!(lines.next().is_some(), "{WORKFLOW} defines no {header}");
    let mut body = Vec::new();
    for line in lines {
        if line.trim() == "}" {
            return body.join("\n");
        }
        body.push(line);
    }
    panic!("{header} is never closed");
}

/// The text of one workflow step: from its `- name:` line to the next step (the
/// next `- ` item at the same indentation, whatever key it starts with).
fn step_text(text: &str, name: &str) -> String {
    let header = format!("- name: {name}");
    let mut lines = text.lines().skip_while(|line| line.trim() != header);
    let first = lines
        .next()
        .unwrap_or_else(|| panic!("{WORKFLOW} has no step {name:?}"));
    let indent = first.len() - first.trim_start().len();
    lines
        .take_while(|line| {
            let own = line.len() - line.trim_start().len();
            !(own == indent && line.trim_start().starts_with("- "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The code lines (no comment lines) of a text.
fn code_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// The `-v` arguments of the code lines of a shell function, unquoted.
fn volume_mounts(body: &str) -> Vec<String> {
    let pattern = Regex::new(r#"-v\s+("[^"]+"|\S+)"#).unwrap();
    code_lines(body)
        .iter()
        .flat_map(|line| {
            pattern
                .captures_iter(line)
                .map(|c| c[1].trim_matches('"').to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn eqv_04_workflow_static_run_vectors_is_offline_and_least_privilege() {
    let text = workflow();
    let body = function_body(&text, "run_vectors");
    for needle in [
        "--network none",
        "--user 65534:65534",
        "--read-only",
        "--cap-drop ALL",
        "no-new-privileges",
        "--pids-limit",
        "--memory",
        "--memory-swap",
        "--cpus",
        "/usr/local/bin/mt:ro",
        "/opt/mt/pins.toml:ro",
        "/opt/mt/testbed:ro",
    ] {
        assert!(body.contains(needle), "run_vectors lacks {needle}");
    }
    for forbidden in ["--privileged", "--cap-add", "seccomp", "docker.sock"] {
        assert!(
            !body.contains(forbidden),
            "run_vectors must not hold {forbidden}"
        );
    }

    // Exactly /work and /out are writable; every other mount is read-only.
    let mounts = volume_mounts(&body);
    let writable: Vec<&String> = mounts.iter().filter(|m| !m.ends_with(":ro")).collect();
    assert_eq!(
        writable.len(),
        2,
        "run_vectors must have exactly two writable mounts: {writable:?}"
    );
    assert!(writable[0].ends_with(":/work"), "{writable:?}");
    assert!(writable[1].ends_with(":/out"), "{writable:?}");

    // The only tmpfs is /tmp, as in the other helpers.
    for line in code_lines(&body) {
        if let Some(at) = line.find("--tmpfs") {
            assert!(
                line[at..].starts_with("--tmpfs /tmp:"),
                "run_vectors mounts a tmpfs other than /tmp: {line}"
            );
        }
    }

    let last = code_lines(&body)
        .into_iter()
        .rfind(|line| !line.trim().is_empty())
        .unwrap();
    assert!(
        last.trim_end().ends_with("mt vectors \"$@\""),
        "run_vectors must end in `mt vectors \"$@\"`: {last}"
    );
}

#[test]
fn eqv_04_workflow_static_selftest_step_publishes_its_notices() {
    let text = workflow();
    let step = step_text(&text, SELFTEST_STEP);
    let code = code_lines(&step).join("\n");
    assert!(
        code.contains("run_vectors selftest"),
        "the step never runs `run_vectors selftest`:\n{code}"
    );
    assert!(
        code.contains("finish_vectors selftest"),
        "the step never calls `finish_vectors selftest`:\n{code}"
    );
    assert!(
        code.contains("fixture vectors-selftest-dwarf"),
        "the step does not publish the DWARF fixture:\n{code}"
    );
    let condition = code
        .lines()
        .find(|line| line.trim_start().starts_with("if:"))
        .unwrap_or_else(|| panic!("the step has no `if:`:\n{code}"));
    assert!(
        condition.contains("steps.image.outcome == 'success'"),
        "the step does not require the image step: {condition}"
    );
}

#[test]
fn eqv_04_workflow_static_vectors_steps_never_publish_testbed_titles() {
    let text = workflow();

    // finish_vectors publishes `vectors-` titles only.
    let finish = function_body(&text, "finish_vectors");
    assert!(
        finish.contains("annotate error \"vectors-$step\"")
            && finish.contains("annotate notice \"vectors-$step\""),
        "finish_vectors must publish vectors-STEP as an error or a notice:\n{finish}"
    );
    assert!(
        !finish.contains("testbed-"),
        "finish_vectors publishes a testbed- title:\n{finish}"
    );

    // The self-test step publishes none either.
    let step = code_lines(&step_text(&text, SELFTEST_STEP)).join("\n");
    assert!(
        !step.contains("testbed-"),
        "the self-test step publishes a testbed- title:\n{step}"
    );

    // Every literal testbed- title in the workflow is one of the eight of Phase 2.
    let pattern = Regex::new(r#"annotate\s+\w+\s+"?(testbed-[a-z]+)"#).unwrap();
    let titles: BTreeSet<String> = code_lines(&text)
        .iter()
        .flat_map(|line| {
            pattern
                .captures_iter(line)
                .map(|c| c[1].to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    for title in &titles {
        assert!(
            PHASE_2_TITLES.contains(&title.as_str()),
            "the workflow publishes a testbed- title of its own: {title}"
        );
    }

    // The header comment lists the new titles.
    let header: String = text
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        header.contains("vectors-selftest"),
        "the header comment does not list vectors-selftest"
    );
    assert!(
        header.contains("fixture vectors-selftest-dwarf"),
        "the header comment does not list fixture vectors-selftest-dwarf"
    );
}
