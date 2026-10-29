#![allow(clippy::unwrap_used, clippy::expect_used)]
//! EQV-04: static checks of the `run_vectors` helper and the self-test step of
//! `.github/workflows/testbed.yml`. They read the repository file and run under
//! plain `cargo test`, with no Docker and no network.
//!
//! The Phase 3 steps live in their own job, `vectors` (CONTEXT D-32), so that
//! they have an annotation budget of their own: the `testbed` job is at GitHub's
//! 50 annotations. Every check of a vectors step or helper reads the text of that
//! job; the `testbed` job keeps the eight Phase 2 steps unchanged.

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

/// The text of the job `name` of `jobs:`: from its `  name:` line to the next
/// job (the next key at two spaces of indentation).
fn job_text(text: &str, name: &str) -> String {
    let header = format!("  {name}:");
    let mut lines = text.lines().skip_while(|line| *line != header);
    assert!(
        lines.next().is_some(),
        "{WORKFLOW} has no job {name:?} under jobs:"
    );
    let mut body = vec![header.clone()];
    for line in lines {
        let is_key = line.starts_with("  ")
            && !line.starts_with("   ")
            && !line.trim_start().starts_with('#')
            && line.trim_end().ends_with(':');
        if is_key {
            break;
        }
        body.push(line.to_owned());
    }
    // The comment above the next job and the blank lines before it are not part
    // of this one.
    while body
        .last()
        .is_some_and(|line| line.trim().is_empty() || line.starts_with("  #"))
    {
        body.pop();
    }
    body.join("\n")
}

/// The text of the `vectors` job (CONTEXT D-32).
fn vectors_job() -> String {
    job_text(&workflow(), "vectors")
}

/// The text of the `testbed` job.
fn testbed_job() -> String {
    job_text(&workflow(), "testbed")
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
    let text = vectors_job();
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
    let text = vectors_job();
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
    let whole = workflow();
    let text = vectors_job();

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

    // The vectors job publishes no testbed- title at all (D-32): its notices are
    // read from its own job id and are never mixed with the Phase 2 baseline.
    let in_job: Vec<String> = code_lines(&text)
        .into_iter()
        .filter(|line| line.contains("annotate") && line.contains("testbed-"))
        .collect();
    assert!(
        in_job.is_empty(),
        "the vectors job publishes a testbed- title: {in_job:?}"
    );

    // Every literal testbed- title in the workflow is one of the eight of Phase 2.
    let pattern = Regex::new(r#"annotate\s+\w+\s+"?(testbed-[a-z]+)"#).unwrap();
    let titles: BTreeSet<String> = code_lines(&whole)
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
    let header: String = whole
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

const TRACTOR_STEP: &str = "Fetch the TRACTOR sample and load it unchanged (local use only)";

/// Every flag, limit and read-only mount of the generic networked helper.
const HELPER_HARDENING: [&str; 12] = [
    "--user 65534:65534",
    "--read-only",
    "--cap-drop ALL",
    "no-new-privileges",
    "--pids-limit 1024",
    "--memory 6g",
    "--memory-swap 6g",
    "--cpus 2",
    "--tmpfs /tmp:rw,exec,nosuid,size=2g",
    "/usr/local/bin/mt:ro",
    "/opt/mt/pins.toml:ro",
    "/opt/mt/testbed:ro",
];

#[test]
fn eqv_02_workflow_static_tractor_fetch_is_the_only_new_networked_step() {
    let whole = workflow();
    let text = vectors_job();
    let fetch = function_body(&text, "fetch_tractor");
    // run_networked is the helper of the testbed job; fetch_tractor must mount the
    // same things (D-32: the vectors job carries no run_networked of its own).
    let networked = function_body(&whole, "run_networked");

    // The same hardening as run_networked, with the network left on.
    for needle in HELPER_HARDENING {
        assert!(networked.contains(needle), "run_networked lacks {needle}");
        assert!(fetch.contains(needle), "fetch_tractor lacks {needle}");
    }
    assert!(
        !fetch.contains("--network"),
        "fetch_tractor must not change the network setting"
    );
    for forbidden in ["--privileged", "--cap-add", "seccomp", "docker.sock"] {
        assert!(
            !fetch.contains(forbidden),
            "fetch_tractor holds {forbidden}"
        );
    }
    assert_eq!(
        volume_mounts(&fetch),
        volume_mounts(&networked),
        "fetch_tractor must mount what run_networked mounts"
    );

    // It can run only the TRACTOR fetch.
    let last = code_lines(&fetch)
        .into_iter()
        .rfind(|line| !line.trim().is_empty())
        .unwrap();
    assert!(
        last.trim_end().ends_with("mt vectors tractor fetch \"$@\""),
        "fetch_tractor must end in `mt vectors tractor fetch \"$@\"`: {last}"
    );
    assert_eq!(
        code_lines(&fetch)
            .iter()
            .filter(|l| l.contains("docker run"))
            .count(),
        1
    );

    // It is called in exactly one step, the TRACTOR step.
    let calls = Regex::new(r"\bfetch_tractor\s+\S").unwrap();
    let call_lines: Vec<String> = code_lines(&whole)
        .into_iter()
        .filter(|line| calls.is_match(line))
        .collect();
    assert_eq!(
        call_lines.len(),
        1,
        "fetch_tractor is called on: {call_lines:?}"
    );
    assert!(
        !testbed_job().contains("fetch_tractor"),
        "the testbed job must not hold the TRACTOR fetch helper (D-32)"
    );
    let step = code_lines(&step_text(&text, TRACTOR_STEP)).join("\n");
    assert!(
        calls.is_match(&step),
        "the call is not in the step {TRACTOR_STEP:?}:\n{step}"
    );
    assert!(step.contains("tractor.txt"), "{step}");
    for needle in ["run_vectors tractor check", "finish_vectors tractor"] {
        assert!(
            step.contains(needle),
            "the TRACTOR step lacks {needle}:\n{step}"
        );
    }
    let condition = step
        .lines()
        .find(|line| line.trim_start().starts_with("if:"))
        .unwrap_or_else(|| panic!("the step has no `if:`:\n{step}"));
    assert!(
        condition.contains("steps.image.outcome == 'success'"),
        "{condition}"
    );
    assert!(!step.contains("continue-on-error"), "{step}");
    assert!(
        step.contains("fetch failed with exit code"),
        "a fetch failure must be reported as such:\n{step}"
    );
    assert!(
        !step.contains("testbed-"),
        "the step publishes a testbed- title"
    );

    // The check stays offline: no run_vectors call names the fetch.
    let offline_fetch = Regex::new(r"run_vectors\s+tractor\s+fetch").unwrap();
    assert!(
        !code_lines(&text).iter().any(|l| offline_fetch.is_match(l)),
        "run_vectors must never run the TRACTOR fetch"
    );
    let vectors = function_body(&text, "run_vectors");
    assert!(
        vectors.contains("--network none"),
        "run_vectors must stay offline"
    );

    // The header comment lists the title and the amended network policy.
    let header: String = whole
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        header.contains("vectors-tractor"),
        "the header lacks vectors-tractor"
    );
    assert!(
        header.contains("fetch_tractor"),
        "the network policy paragraph does not name fetch_tractor"
    );
}

const BUILD_STEP: &str = "Build the reference drivers";
const RUN_STEP: &str = "Run the reference vectors twice";
const DETERMINISM_STEP: &str = "Run the vectors again and compare (vectors determinism)";
const RESTORE_STEP: &str = "Unpack the captures of the testbed job";

#[test]
fn eqv_04_workflow_static_run_guest_cannot_write_the_evidence() {
    let whole = workflow();
    let text = vectors_job();
    let body = function_body(&text, "run_guest");
    let vectors = function_body(&text, "run_vectors");

    // The same flags, limits and read-only mounts as run_vectors, and no network.
    for needle in HELPER_HARDENING {
        assert!(vectors.contains(needle), "run_vectors lacks {needle}");
        assert!(body.contains(needle), "run_guest lacks {needle}");
    }
    assert!(
        body.contains("--network none"),
        "run_guest needs no network"
    );
    for forbidden in ["--privileged", "--cap-add", "seccomp", "docker.sock"] {
        assert!(
            !body.contains(forbidden),
            "run_guest must not hold {forbidden}"
        );
    }

    // /work is read-only; the one writable mount is the run's own output
    // directory under $RUNNER_TEMP/out, at /vectors.
    let mounts = volume_mounts(&body);
    assert!(
        mounts.iter().any(|m| m.ends_with(":/work:ro")),
        "run_guest must mount /work read-only: {mounts:?}"
    );
    assert!(
        !mounts.iter().any(|m| m.ends_with(":/out")),
        "run_guest must not mount /out: {mounts:?}"
    );
    let writable: Vec<&String> = mounts.iter().filter(|m| !m.ends_with(":ro")).collect();
    assert_eq!(
        writable.len(),
        1,
        "run_guest must have exactly one writable bind mount: {writable:?}"
    );
    assert!(writable[0].ends_with(":/vectors"), "{writable:?}");
    assert!(
        writable[0].starts_with("$RUNNER_TEMP/out/"),
        "the writable source must lie under $RUNNER_TEMP/out/: {writable:?}"
    );

    // The slots live in a tmpfs at /tmp, the only tmpfs.
    let tmpfs: Vec<String> = code_lines(&body)
        .into_iter()
        .filter(|line| line.contains("--tmpfs"))
        .collect();
    assert_eq!(tmpfs.len(), 1, "{tmpfs:?}");
    assert!(tmpfs[0].contains("--tmpfs /tmp:"), "{tmpfs:?}");

    // It takes the output name first and ends in `mt vectors "$@"`.
    let code = code_lines(&body);
    assert!(
        code.iter().any(|line| line.contains("shift")),
        "run_guest must take the output name and shift it away:\n{body}"
    );
    let last = code
        .into_iter()
        .rfind(|line| !line.trim().is_empty())
        .unwrap();
    assert!(
        last.trim_end().ends_with("mt vectors \"$@\""),
        "run_guest must end in `mt vectors \"$@\"`: {last}"
    );
    // The comment above it says what it is for.
    assert!(
        whole.contains("QEMU user mode passes guest syscalls"),
        "the run_guest comment must state the syscall boundary"
    );
}

#[test]
fn eqv_04_workflow_static_vectors_steps_run_in_order() {
    let whole = workflow();
    let text = vectors_job();
    let restore_at = text
        .find(&format!("- name: {RESTORE_STEP}"))
        .expect("the step that unpacks the captures exists");
    let mut last = restore_at;
    for name in [BUILD_STEP, RUN_STEP, DETERMINISM_STEP] {
        let at = text
            .find(&format!("- name: {name}"))
            .unwrap_or_else(|| panic!("{WORKFLOW} has no step {name:?}"));
        assert!(at > last, "{name:?} must come after the previous step");
        last = at;
    }
    let tractor_at = text
        .find(&format!("- name: {TRACTOR_STEP}"))
        .expect("the TRACTOR step exists");
    assert!(
        tractor_at < text.find(&format!("- name: {BUILD_STEP}")).unwrap(),
        "the vectors steps come after the TRACTOR step"
    );

    let build = code_lines(&step_text(&text, BUILD_STEP)).join("\n");
    assert!(build.contains("run_vectors build"), "{build}");
    assert!(build.contains("--replay-dir /out/replay"), "{build}");
    assert!(build.contains("finish_vectors build"), "{build}");

    let run = code_lines(&step_text(&text, RUN_STEP)).join("\n");
    assert!(run.contains("run_guest vectors-run run"), "{run}");
    assert!(run.contains("finish_vectors run"), "{run}");

    let determinism = code_lines(&step_text(&text, DETERMINISM_STEP)).join("\n");
    assert!(
        determinism.contains("run_guest vectors-rerun run"),
        "{determinism}"
    );
    assert!(
        determinism
            .contains("run_vectors check --first /out/vectors-run --second /out/vectors-rerun"),
        "{determinism}"
    );
    assert!(
        determinism.contains("finish_vectors determinism"),
        "{determinism}"
    );

    for (name, step) in [
        (BUILD_STEP, &build),
        (RUN_STEP, &run),
        (DETERMINISM_STEP, &determinism),
    ] {
        let condition = step
            .lines()
            .find(|line| line.trim_start().starts_with("if:"))
            .unwrap_or_else(|| panic!("{name:?} has no `if:`:\n{step}"));
        assert!(
            condition.contains("steps.restore.outcome == 'success'"),
            "{name:?} must require the step that unpacks the captures: {condition}"
        );
        assert!(!step.contains("continue-on-error"), "{name:?}: {step}");
        assert!(
            !step.contains("testbed-"),
            "{name:?} publishes a testbed- title"
        );
        // The rerun and the output directories exist, writable by the
        // container user, before they are mounted.
        assert!(
            !step.contains("docker run"),
            "{name:?} starts a container outside a helper"
        );
    }
    assert!(
        run.contains("chmod 0777") && determinism.contains("chmod 0777"),
        "the output directories are created for the container user"
    );

    let header: String = whole
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "vectors-build",
        "vectors-run",
        "vectors-determinism",
        "run_guest",
    ] {
        assert!(header.contains(needle), "the header lacks {needle}");
    }
}

#[test]
fn eqv_04_workflow_static_captured_code_runs_only_in_run_guest() {
    let text = workflow();
    let code = code_lines(&text);
    // No run_vectors call names a subcommand that runs captured object code.
    let captured = Regex::new(r"\brun_vectors\s+(run|replay)\b").unwrap();
    let offending: Vec<&String> = code.iter().filter(|l| captured.is_match(l)).collect();
    assert!(
        offending.is_empty(),
        "captured object code runs only in run_guest: {offending:?}"
    );
    // run_guest runs nothing but `run` and `replay`.
    let guest = Regex::new(r"\brun_guest\s+(\S+)\s+(\S+)").unwrap();
    let mut calls = 0;
    for line in &code {
        for call in guest.captures_iter(line) {
            calls += 1;
            assert!(
                matches!(&call[2], "run" | "replay"),
                "run_guest is called with {} on: {line}",
                &call[2]
            );
            assert!(
                matches!(
                    &call[1],
                    "vectors-run" | "vectors-rerun" | "vectors-sample-replay"
                ),
                "run_guest writes to {} on: {line}",
                &call[1]
            );
            // `replay` runs the committed sample and nothing else.
            assert_eq!(
                &call[2] == "replay",
                &call[1] == "vectors-sample-replay",
                "{line}"
            );
        }
    }
    assert_eq!(
        calls, 3,
        "run_guest is called three times: two corpora and the committed sample"
    );
    // Only the self-test, the TRACTOR check, the build, the layout listing and
    // the comparison go through run_vectors.
    let allowed = Regex::new(r"\brun_vectors\s+(\S+)").unwrap();
    for line in &code {
        for call in allowed.captures_iter(line) {
            assert!(
                matches!(
                    &call[1],
                    "selftest" | "tractor" | "build" | "check" | "layouts" | "sample"
                ),
                "unexpected run_vectors subcommand {} on: {line}",
                &call[1]
            );
        }
    }
}

const LAYOUT_STEP: &str = "Publish the state layouts";

#[test]
fn eqv_04_workflow_static_layout_step_follows_the_build() {
    let whole = workflow();
    let text = vectors_job();
    let build_at = text
        .find(&format!("- name: {BUILD_STEP}"))
        .expect("the build step exists");
    let layout_at = text
        .find(&format!("- name: {LAYOUT_STEP}"))
        .unwrap_or_else(|| panic!("{WORKFLOW} has no step {LAYOUT_STEP:?}"));
    assert!(layout_at > build_at, "the layouts come after the build");
    let run_at = text
        .find(&format!("- name: {RUN_STEP}"))
        .expect("the run step exists");
    assert!(
        layout_at < run_at,
        "the layouts are published before the run"
    );

    let step = step_text(&text, LAYOUT_STEP);
    let code = code_lines(&step).join("\n");
    assert!(
        code.contains("run_vectors layouts --work-dir /work"),
        "the step never runs `run_vectors layouts --work-dir /work`:\n{code}"
    );
    assert!(
        code.contains("finish_vectors layout"),
        "the step never calls `finish_vectors layout`:\n{code}"
    );
    let condition = code
        .lines()
        .find(|line| line.trim_start().starts_with("if:"))
        .unwrap_or_else(|| panic!("the step has no `if:`:\n{code}"));
    assert!(
        condition.contains("steps.vbuild.outcome == 'success'"),
        "the step does not require the build step: {condition}"
    );
    assert!(!code.contains("continue-on-error"), "{code}");
    assert!(
        !code.contains("testbed-"),
        "the step publishes a testbed- title"
    );
    assert!(
        !code.contains("docker run"),
        "the step starts a container outside a helper"
    );
    assert!(
        !code.contains("run_guest"),
        "the layouts read build.json only and run nothing from upstream"
    );

    let header: String = whole
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        header.contains("vectors-layout"),
        "the header comment does not list vectors-layout"
    );
}

/// The eight steps of Phase 2 in the `testbed` job, in order, with the sha256 of
/// the text of each as `step_text` returns it (CONTEXT D-32: they stay byte-identical
/// when the vectors steps move to their own job). The hashes were taken from the
/// workflow at commit ef0bdbd, before the move. Changing one of these steps means
/// changing this table on purpose, in the commit that does it.
const PHASE_2_STEPS: [(&str, &str); 8] = [
    (
        "Pull the image by digest",
        "f9ef0739bc2238675b2d9d3b85a6af4200d7dda9143aa099a5bdf2efeea3ed7a",
    ),
    (
        "Validate the manifest",
        "96402941942a26e376c9ee2e61fa5e6303cbcf80ac8c159bcc256cbf77a51b73",
    ),
    (
        "Fetch the sources",
        "a7fa459ad587207dc1ec3f278c04b30bf0c741b884bcb28457a7a3d9937e033b",
    ),
    (
        "Check the committed seeds",
        "d2df256006bd16ba3c6150c8eba8d8af28ce287a5d82e994797122cff88af871",
    ),
    (
        "Build each pair in its own container",
        "245334faf77b9f10f6b1e2774850eda6762c2d3b809d8088cc10b7cdbab17b9f",
    ),
    (
        "Record the captures",
        "772d1e6c40e99cb115858338f1c6f026319665990beb17326e502f6d40546b21",
    ),
    (
        "Replay the captured builds",
        "90e3a5b5fd44aaa12f6a1abe52c99aba4a8d5cf65d1dc0c3d5cd2be84545fd7f",
    ),
    (
        "Capture again and compare the reports (determinism)",
        "78d360c5b0e13fbf1ca62c1ad88717fdfdb225a285d2825bfdb4ef278b00af84",
    ),
];

#[test]
fn eqv_04_workflow_static_phase_2_steps_are_unchanged() {
    let testbed = testbed_job();
    let mut seen = Vec::new();
    for (name, _) in PHASE_2_STEPS {
        seen.push((
            name,
            mt_vectors::sha256_hex(step_text(&testbed, name).as_bytes()),
        ));
    }
    let expected: Vec<(&str, String)> = PHASE_2_STEPS
        .iter()
        .map(|(name, hash)| (*name, (*hash).to_owned()))
        .collect();
    assert_eq!(seen, expected, "a Phase 2 step of the testbed job changed");

    // They stay in this order, and the job publishes exactly the eight titles.
    let positions: Vec<usize> = PHASE_2_STEPS
        .iter()
        .map(|(name, _)| testbed.find(&format!("- name: {name}")).unwrap())
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{positions:?}"
    );
    let pattern = Regex::new(r#"annotate\s+\w+\s+"?(testbed-[a-z]+)"#).unwrap();
    let finish = Regex::new(r"\bfinish_step\s+([a-z]+)\s").unwrap();
    let titles: BTreeSet<String> = code_lines(&testbed)
        .iter()
        .flat_map(|line| {
            let mut found: Vec<String> = pattern
                .captures_iter(line)
                .map(|c| c[1].to_owned())
                .collect();
            found.extend(
                finish
                    .captures_iter(line)
                    .map(|c| format!("testbed-{}", &c[1])),
            );
            found
        })
        .collect();
    assert_eq!(
        titles.into_iter().collect::<Vec<_>>(),
        {
            let mut all: Vec<String> = PHASE_2_TITLES.iter().map(|t| (*t).to_owned()).collect();
            all.sort();
            all
        },
        "the testbed job publishes the eight Phase 2 titles and no other"
    );
}

#[test]
fn eqv_04_workflow_static_vectors_job_is_separate_and_equally_hardened() {
    let whole = workflow();
    let testbed = testbed_job();
    let vectors = vectors_job();

    // The vectors job runs after the bundle and the testbed job, on the same runner
    // image, and runs even when the testbed job failed (it has its own evidence).
    let key_lines = |text: &str, key: &str| -> Vec<String> {
        code_lines(text)
            .into_iter()
            .filter(|line| line.starts_with(&format!("    {key}:")))
            .collect()
    };
    let needs = key_lines(&vectors, "needs");
    assert_eq!(needs.len(), 1, "{needs:?}");
    assert!(
        needs[0].contains("mt") && needs[0].contains("testbed"),
        "the vectors job needs the mt bundle and the testbed job: {needs:?}"
    );
    assert_eq!(
        key_lines(&vectors, "runs-on"),
        key_lines(&testbed, "runs-on"),
        "the two jobs run on the same runner image"
    );
    assert_eq!(key_lines(&vectors, "timeout-minutes").len(), 1);
    let condition = key_lines(&vectors, "if");
    assert_eq!(condition.len(), 1, "{condition:?}");
    assert!(
        condition[0].contains("!cancelled()")
            && condition[0].contains("needs.mt.result == 'success'"),
        "the vectors job runs when the testbed job failed, never without the bundle: {condition:?}"
    );

    // None of the Phase 3 steps or helpers is left in the testbed job, and none of
    // the Phase 2 helpers that run upstream code is in the vectors job.
    for needle in [
        "run_vectors",
        "run_guest",
        "fetch_tractor",
        "finish_vectors",
        "vectors-",
    ] {
        let hits: Vec<String> = code_lines(&testbed)
            .into_iter()
            .filter(|line| line.contains(needle) && !line.contains("vectors-input"))
            .collect();
        assert!(
            hits.is_empty(),
            "the testbed job still holds {needle}: {hits:?}"
        );
    }
    for needle in ["run_networked", "run_offline", "run_build", "finish_step"] {
        let hits: Vec<String> = code_lines(&vectors)
            .into_iter()
            .filter(|line| line.contains(needle))
            .collect();
        assert!(
            hits.is_empty(),
            "the vectors job holds the Phase 2 helper {needle}: {hits:?}"
        );
    }

    // The same image, only by the digest of container/README.md, pulled in the job.
    let pull = step_text(&vectors, "Pull the image by digest");
    assert!(pull.contains("container/README.md"), "{pull}");
    assert!(pull.contains("docker pull"), "{pull}");
    assert!(pull.contains("annotate notice vectors-image"), "{pull}");
    assert!(
        !pull.contains("testbed-"),
        "the vectors job publishes no testbed- title"
    );
    assert!(
        !Regex::new(r"sha256:[0-9a-f]{64}")
            .unwrap()
            .is_match(&vectors),
        "the vectors job must not hold a digest of its own"
    );

    // Every container of the job is started by one of its helpers, none by a step,
    // and the helpers keep the hardening of run_vectors.
    let outside = {
        let mut kept = Vec::new();
        let mut skipping = false;
        for line in vectors.lines() {
            if ["image_tool", "run_vectors", "run_guest", "fetch_tractor"]
                .iter()
                .any(|name| line.trim() == format!("{name}() {{"))
            {
                skipping = true;
            }
            if !skipping {
                kept.push(line);
            } else if line.trim() == "}" {
                skipping = false;
            }
        }
        kept.join("\n")
    };
    assert!(
        !code_lines(&outside)
            .iter()
            .any(|line| line.contains("docker run")),
        "a step of the vectors job starts a container outside a helper"
    );
    for name in ["image_tool", "run_vectors", "run_guest"] {
        let body = function_body(&vectors, name);
        assert!(body.contains("--network none"), "{name} needs no network");
        for needle in HELPER_HARDENING {
            // image_tool runs a tool of the image, not mt: it has no mt, pins or
            // testbed mount.
            if name == "image_tool" && needle.starts_with('/') {
                continue;
            }
            assert!(body.contains(needle), "{name} lacks {needle}");
        }
    }
    assert!(!vectors.contains("docker.sock") && !vectors.contains("secrets."));
    assert!(
        whole.contains("D-32"),
        "the header comment records the decision D-32"
    );

    // Every checkout of the vectors job drops the token.
    let lines: Vec<&str> = vectors.lines().collect();
    let checkout = lines
        .iter()
        .position(|line| line.contains("uses: actions/checkout@"))
        .expect("the vectors job checks the repository out");
    assert!(
        lines[checkout..lines.len().min(checkout + 4)]
            .join("\n")
            .contains("persist-credentials: false")
    );
}

#[test]
fn eqv_04_workflow_static_captures_travel_by_artifact() {
    // The vectors steps read the verified captures, the builds and the replay
    // reports of the testbed job. They travel as two tar files made by an image
    // tool (the files belong to the container user), uploaded once and downloaded
    // in the vectors job; no step of either job fetches or rebuilds them.
    let testbed = testbed_job();
    let vectors = vectors_job();

    let archive = step_text(&testbed, "Archive the captures for the vectors job");
    let code = code_lines(&archive).join("\n");
    assert!(code.contains("image_tool"), "{code}");
    assert!(code.contains("tar "), "{code}");
    for member in ["capture", "run"] {
        assert!(
            Regex::new(&format!(r"-cf /out/work\.tar\b.*\b{member}\b"))
                .unwrap()
                .is_match(&code),
            "the work archive lacks {member}:\n{code}"
        );
    }
    assert!(code.contains("/out/replay.tar"), "{code}");
    let condition = code
        .lines()
        .find(|line| line.trim_start().starts_with("if:"))
        .unwrap_or_else(|| panic!("the step has no `if:`:\n{code}"));
    assert!(
        condition.contains("steps.replay.outcome == 'success'"),
        "the archive needs the replay step: {condition}"
    );
    assert!(!code.contains("testbed-"), "{code}");

    let upload = testbed
        .split("- uses: actions/upload-artifact@")
        .filter(|chunk| chunk.contains("name: vectors-input"))
        .count();
    assert_eq!(upload, 1, "the testbed job uploads vectors-input once");
    let upload_at = testbed.find("name: vectors-input").unwrap();
    assert!(
        upload_at
            > testbed
                .find("- name: Archive the captures for the vectors job")
                .unwrap(),
        "the upload follows the archive"
    );

    let download = step_text(&vectors, "Download the captures of the testbed job");
    let download_code = code_lines(&download).join("\n");
    assert!(
        download_code.contains("uses: actions/download-artifact@"),
        "{download_code}"
    );
    assert!(
        download_code.contains("name: vectors-input"),
        "{download_code}"
    );
    assert!(download_code.contains("id: handoff"), "{download_code}");
    let restore = step_text(&vectors, RESTORE_STEP);
    let restore_code = code_lines(&restore).join("\n");
    assert!(restore_code.contains("id: restore"), "{restore_code}");
    assert!(
        restore_code.contains("steps.handoff.outcome == 'success'"),
        "{restore_code}"
    );
    assert!(restore_code.contains("tar -xf"), "{restore_code}");
    assert!(
        !restore_code.contains("tar -xPf") && !restore_code.contains("--absolute-names"),
        "the archives are unpacked with relative names only"
    );
    // Only the unpacked trees are made readable. The work and out directories also
    // hold files the earlier steps' containers made (owned by the container user),
    // which the runner may not chmod: a recursive chmod over them fails the step
    // (found in round trip 1 of plan 03-08).
    let chmod = restore_code
        .lines()
        .find(|line| line.contains("chmod -R a+rX"))
        .unwrap_or_else(|| panic!("the unpacked files are not made readable:\n{restore_code}"));
    for tree in [
        "\"$RUNNER_TEMP/work/capture\"",
        "\"$RUNNER_TEMP/work/run\"",
        "\"$RUNNER_TEMP/out/replay\"",
    ] {
        assert!(chmod.contains(tree), "chmod misses {tree}: {chmod}");
    }
    for whole in ["\"$RUNNER_TEMP/work\"", "\"$RUNNER_TEMP/out\""] {
        assert!(
            !chmod.contains(&format!("{whole} ")) && !chmod.trim_end().ends_with(whole),
            "chmod covers all of {whole}: {chmod}"
        );
    }
    assert!(!restore_code.contains("docker run"), "{restore_code}");
}

const SAMPLE_FIXTURE_STEP: &str = "Fixture vectors sample";
const SAMPLE_REPLAY_STEP: &str = "Replay the committed sample";

#[test]
fn eqv_04_workflow_static_sample_steps() {
    // D-21, T-03-29: the sample is made by CI in the pinned image and carried to the
    // repository through a fixture notice whose tar sha256 is checked before commit;
    // the next run replays the committed sample byte for byte.
    let whole = workflow();
    let text = vectors_job();
    let run_at = text.find(&format!("- name: {RUN_STEP}")).unwrap();
    let determinism_at = text.find(&format!("- name: {DETERMINISM_STEP}")).unwrap();
    let fixture_at = text
        .find(&format!("- name: {SAMPLE_FIXTURE_STEP}\n"))
        .unwrap_or_else(|| panic!("{WORKFLOW} has no step {SAMPLE_FIXTURE_STEP:?}"));
    let replay_at = text
        .find(&format!("- name: {SAMPLE_REPLAY_STEP}"))
        .unwrap_or_else(|| panic!("{WORKFLOW} has no step {SAMPLE_REPLAY_STEP:?}"));
    assert!(
        run_at < fixture_at && determinism_at < fixture_at && run_at < replay_at,
        "both sample steps come after the run and the determinism steps"
    );

    // The fixture step: select, archive deterministically in an image tool, compress,
    // encode and publish with the tar's sha256 as the last line.
    let fixture = code_lines(&step_text(&text, SAMPLE_FIXTURE_STEP)).join("\n");
    for needle in [
        "run_vectors sample",
        "--corpus-dir /out/vectors-run",
        "--out /out/vectors-sample",
        "image_tool",
        "--sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner",
        "gzip -n -9",
        "base64",
        "tar sha256",
        "annotate notice \"fixture vectors-sample\"",
    ] {
        assert!(
            fixture.contains(needle),
            "the fixture step lacks {needle:?}:\n{fixture}"
        );
    }
    let condition = fixture
        .lines()
        .find(|line| line.trim_start().starts_with("if:"))
        .unwrap_or_else(|| panic!("the step has no `if:`:\n{fixture}"));
    assert!(
        condition.contains("steps.restore.outcome == 'success'")
            && condition.contains("steps.vrun.outcome == 'success'"),
        "{condition}"
    );
    assert!(!fixture.contains("continue-on-error") && !fixture.contains("testbed-"));
    assert!(!fixture.contains("docker run"), "{fixture}");
    // The last line of the notice is the hash line, then the parts are published
    // in ranges of at most nine, as the other long fixtures are.
    let sha_line = fixture.find("tar sha256").unwrap();
    let encode = fixture.find("base64").unwrap();
    assert!(
        encode < sha_line,
        "the hash line comes after the encoded tar"
    );
    let ranges = Regex::new(r#"annotate notice "fixture vectors-sample" \S+ (\d+) (\d+)"#).unwrap();
    let mut published = Vec::new();
    for name in [
        SAMPLE_FIXTURE_STEP,
        "Fixture vectors sample (parts 10 to 18)",
        "Fixture vectors sample (parts 19 on)",
    ] {
        let step = code_lines(&step_text(&text, name)).join("\n");
        let caps = ranges
            .captures(&step)
            .unwrap_or_else(|| panic!("{name:?} publishes no part range:\n{step}"));
        published.push((
            caps[1].parse::<u32>().unwrap(),
            caps[2].parse::<u32>().unwrap(),
        ));
        assert!(!step.contains("docker run"), "{name:?}");
    }
    assert_eq!(published, [(1, 9), (10, 18), (19, 27)]);

    // The replay step: the committed sample, the image's drivers, no network, and
    // `no committed sample` while the directory does not exist.
    let replay = code_lines(&step_text(&text, SAMPLE_REPLAY_STEP)).join("\n");
    for needle in [
        "run_guest vectors-sample-replay replay",
        "--tree /opt/mt/testbed/vectors-sample",
        "--scratch-dir /tmp/mt-slots",
        "--work-dir /work",
        "finish_vectors sample",
        "no committed sample",
        "chmod 0777",
    ] {
        assert!(
            replay.contains(needle),
            "the replay step lacks {needle:?}:\n{replay}"
        );
    }
    let condition = replay
        .lines()
        .find(|line| line.trim_start().starts_with("if:"))
        .unwrap_or_else(|| panic!("the step has no `if:`:\n{replay}"));
    assert!(
        condition.contains("steps.restore.outcome == 'success'")
            && condition.contains("steps.vbuild.outcome == 'success'"),
        "{condition}"
    );
    assert!(!replay.contains("continue-on-error") && !replay.contains("testbed-"));
    assert!(!replay.contains("docker run"), "{replay}");

    // The header comment lists both titles.
    let header: String = whole
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for needle in ["fixture vectors-sample", "vectors-sample"] {
        assert!(header.contains(needle), "the header lacks {needle}");
    }
}
