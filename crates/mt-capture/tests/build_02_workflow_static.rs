#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-02 and CONTEXT D-12, D-25: static checks of the `testbed` workflow and
//! its documents. They read repository files and run under plain `cargo test`,
//! with no Docker and no network. SHA pinning of `uses:` is already enforced for
//! every workflow by `tool_03_workflows_pin_actions_by_commit_sha`.

use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::capture::{CONFIGURE_HASH_EXCLUSIONS, NORMALISED_CONFIGURE_FILES};
use mt_capture::compare::DETERMINISM_EXCLUSIONS;
use mt_capture::manifest::Manifest;
use regex::Regex;

const WORKFLOW: &str = ".github/workflows/testbed.yml";

/// The subcommands of `mt testbed` a workflow step may name.
const SUBCOMMANDS: [&str; 6] = ["validate", "fetch", "capture", "replay", "seeds", "compare"];

/// The resource limits every container helper carries (WR-04, T-02-57).
const RESOURCE_LIMITS: [&str; 4] = ["--pids-limit", "--memory", "--memory-swap", "--cpus"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Read a repository file, failing the test (never skipping) when it is absent.
fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
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

/// `text` with the bodies of the named functions removed.
fn without_functions(text: &str, names: &[&str]) -> String {
    let mut kept = Vec::new();
    let mut skipping = false;
    for line in text.lines() {
        if names
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
}

/// Every `(helper, subcommand)` a non-comment line invokes.
fn invocations(text: &str) -> Vec<(String, String)> {
    let pattern = Regex::new(r"\b(run_offline|run_networked)\s+(\S+)").unwrap();
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .flat_map(|line| {
            pattern
                .captures_iter(line)
                .map(|c| (c[1].to_owned(), c[2].to_owned()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn build_02_workflow_runs_capture_and_replay_without_network() {
    let text = read_repo_file(WORKFLOW);
    let offline = function_body(&text, "run_offline");
    let networked = function_body(&text, "run_networked");
    let tool = function_body(&text, "image_tool");

    assert!(
        offline.contains("--network none"),
        "run_offline must add --network none"
    );
    assert!(
        !networked.contains("--network"),
        "run_networked must not change the network setting"
    );
    assert!(
        tool.contains("--network none"),
        "image_tool runs build tools and must have no network"
    );

    for (name, body) in [("run_offline", &offline), ("run_networked", &networked)] {
        for needle in [
            "--user 65534:65534",
            "--read-only",
            "--cap-drop ALL",
            "no-new-privileges",
            "/usr/local/bin/mt:ro",
            "/opt/mt/pins.toml:ro",
            "/opt/mt/testbed:ro",
        ] {
            assert!(body.contains(needle), "{name} lacks {needle}");
        }
    }
    for needle in [
        "--user 65534:65534",
        "--read-only",
        "--cap-drop ALL",
        "no-new-privileges",
    ] {
        assert!(tool.contains(needle), "image_tool lacks {needle}");
    }

    // Every `docker run` lives in one of the three helpers: a step cannot start a
    // container with network access by writing its own command.
    let outside = without_functions(&text, &["run_offline", "run_networked", "image_tool"]);
    assert!(
        !outside
            .lines()
            .any(|line| { !line.trim_start().starts_with('#') && line.contains("docker run") }),
        "docker run outside the three helpers"
    );

    let calls = invocations(&text);
    for (helper, subcommand) in &calls {
        assert!(
            SUBCOMMANDS.contains(&subcommand.as_str()),
            "{helper} is invoked without an `mt testbed` subcommand: {subcommand}"
        );
        if subcommand == "fetch" {
            assert_eq!(helper, "run_networked", "fetch is the one networked step");
        } else {
            assert_eq!(
                helper, "run_offline",
                "{subcommand} must run with --network none"
            );
        }
    }
    for subcommand in ["validate", "fetch", "capture", "replay", "compare"] {
        assert!(
            calls.iter().any(|(_, name)| name == subcommand),
            "the workflow never runs {subcommand}"
        );
    }

    // A runaway build or fork loop is bounded by the container, not only by the
    // 45-minute job timeout (WR-04, T-02-57).
    for (name, body) in [
        ("run_offline", &offline),
        ("run_networked", &networked),
        ("image_tool", &tool),
    ] {
        for needle in RESOURCE_LIMITS {
            assert!(body.contains(needle), "{name} lacks {needle}");
        }
    }

    assert!(
        !text.contains("docker.sock"),
        "the Docker socket is mounted"
    );
    assert!(
        !text.contains("secrets."),
        "the workflow reads a secret; none exists or is needed"
    );
}

/// The lines indented by exactly `indent` spaces that start a key, up to the
/// next such key: `(key line, block lines)` pairs of the block after `header`.
fn blocks_under(text: &str, header: &str, indent: usize) -> Vec<(String, Vec<String>)> {
    let mut lines = text.lines().skip_while(|line| *line != header);
    assert!(lines.next().is_some(), "{WORKFLOW} has no `{header}`");
    let prefix = " ".repeat(indent);
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    for line in lines {
        let top_level = !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#');
        if top_level {
            break;
        }
        let is_key = line.starts_with(&prefix)
            && !line[indent..].starts_with(' ')
            && !line[indent..].starts_with('#')
            && line.trim_end().ends_with(':');
        if is_key {
            blocks.push((line.trim().to_owned(), Vec::new()));
        } else if let Some((_, body)) = blocks.last_mut() {
            body.push(line.to_owned());
        }
    }
    blocks
}

#[test]
fn build_02_workflow_is_least_privilege_and_scheduled() {
    let text = read_repo_file(WORKFLOW);

    // Top-level permissions: contents read and nothing else.
    let permissions: Vec<String> = text
        .lines()
        .skip_while(|line| *line != "permissions:")
        .skip(1)
        .take_while(|line| line.starts_with(' '))
        .map(|line| line.trim().to_owned())
        .collect();
    assert_eq!(permissions, ["contents: read"]);
    assert!(
        !text.contains("packages: write") && !text.contains(": write"),
        "the workflow asks for a write permission"
    );

    // Every checkout drops the token from the git configuration.
    let lines: Vec<&str> = text.lines().collect();
    let checkouts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("uses: actions/checkout@"))
        .map(|(at, _)| at)
        .collect();
    assert!(!checkouts.is_empty(), "no checkout found");
    for at in checkouts {
        let window = lines[at..lines.len().min(at + 4)].join("\n");
        assert!(
            window.contains("persist-credentials: false"),
            "checkout at line {} keeps credentials",
            at + 1
        );
    }

    // Every job has a timeout.
    let jobs = blocks_under(&text, "jobs:", 2);
    assert!(
        jobs.len() >= 2,
        "expected the mt and testbed jobs: {jobs:?}"
    );
    for (name, body) in &jobs {
        assert!(
            body.iter()
                .any(|line| line.trim_start().starts_with("timeout-minutes:")),
            "job {name} has no timeout-minutes"
        );
    }

    // Triggers: weekly schedule, manual dispatch, pushes that change the bed or the crates.
    let triggers = blocks_under(&text, "on:", 2);
    let names: Vec<&str> = triggers.iter().map(|(name, _)| name.as_str()).collect();
    for wanted in ["push:", "workflow_dispatch:", "schedule:"] {
        assert!(names.contains(&wanted), "no {wanted} trigger in {names:?}");
    }
    let push_paths: Vec<String> = triggers
        .iter()
        .find(|(name, _)| name == "push:")
        .map(|(_, body)| body.iter().map(|line| line.trim().to_owned()).collect())
        .unwrap();
    for path in ["- testbed/**", "- crates/**"] {
        assert!(
            push_paths.iter().any(|line| line == path),
            "the push trigger lacks {path}: {push_paths:?}"
        );
    }
    assert!(text.contains("cron:"), "the schedule has no cron entry");
}

#[test]
fn build_02_image_digest_matches_readme() {
    let readme = read_repo_file("container/README.md");
    let pattern =
        Regex::new(r"ghcr\.io/sat-wik/migrationtool-toolchain@(sha256:[0-9a-f]{64})").unwrap();
    let mut digests: Vec<String> = pattern
        .captures_iter(&readme)
        .map(|c| c[1].to_owned())
        .collect();
    digests.sort();
    digests.dedup();
    assert_eq!(
        digests.len(),
        1,
        "container/README.md must hold exactly one pullable digest: {digests:?}"
    );

    let manifest = Manifest::load(&repo_root().join("testbed/manifest.toml")).unwrap();
    assert_eq!(
        manifest.image.digest, digests[0],
        "testbed/manifest.toml [image] digest differs from the README's Current image"
    );

    // The workflow reads the digest from the README; it carries no copy of its own.
    let workflow = read_repo_file(WORKFLOW);
    assert!(workflow.contains("container/README.md"));
    assert!(
        !Regex::new(r"sha256:[0-9a-f]{64}")
            .unwrap()
            .is_match(&workflow),
        "the workflow must not hold a digest of its own"
    );
}

#[test]
fn build_02_testbed_docs_make_no_certification_claim() {
    let readme = read_repo_file("testbed/README.md").to_lowercase();
    for word in ["certified", "certify", "compliant", "compliance"] {
        assert!(
            !readme.contains(word),
            "testbed/README.md contains the forbidden word {word:?}"
        );
    }
    assert!(
        readme.contains("--network none"),
        "the README must show the offline container step"
    );
}

/// The text of one workflow step: from its `- name:` line to the next step.
fn step_text(text: &str, name: &str) -> String {
    let header = format!("- name: {name}");
    let mut lines = text.lines().skip_while(|line| line.trim() != header);
    assert!(lines.next().is_some(), "{WORKFLOW} has no step {name:?}");
    lines
        .take_while(|line| !line.trim_start().starts_with("- name:"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn build_02_workflow_fixture_object_pick_reads_the_whole_listing() {
    // With `--all` the listing of every captured object is longer than a pipe
    // buffer. `... | sort | head -n 1` under `pipefail` then fails when the runner
    // ignores SIGPIPE: `sort` reports "write failed: Broken pipe" and exits 2
    // (testbed run B, step "Make the two Arm fixture objects"). The pick must
    // read the whole listing.
    let text = read_repo_file(WORKFLOW);
    let step = step_text(&text, "Make the two Arm fixture objects");
    let code: Vec<&str> = step
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect();
    assert!(
        code.iter().any(|line| line.contains("small=$(")),
        "the step no longer picks the smallest object"
    );
    for line in code {
        assert!(
            !line.contains("| head") && !line.contains("|head"),
            "a pipeline closes its pipe early under pipefail: {line}"
        );
    }
}

const DETERMINISM_STEP: &str = "Capture again and compare the reports (determinism)";

/// The code lines (comments dropped) of the step `name`.
fn step_code(text: &str, name: &str) -> Vec<String> {
    step_text(text, name)
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .map(str::to_owned)
        .collect()
}

#[test]
fn build_02_workflow_determinism_step_uses_the_fail_closed_comparison() {
    // WR-05: `jq | cmp -s` reported "identical" when jq failed on both files. The
    // verdict is now the exit code of `mt testbed compare` (plan 02-14), which
    // treats unparseable, empty, one-sided and absent files as differences.
    let text = read_repo_file(WORKFLOW);
    let code = step_code(&text, DETERMINISM_STEP);
    let joined = code.join("\n");

    assert!(
        joined.contains(
            "run_offline compare --first /out/capture --second /out/capture-rerun/capture"
        ),
        "the determinism step does not compare through run_offline compare:\n{joined}"
    );
    assert!(
        joined.contains("run_offline capture ") && joined.contains("--out /out/capture-rerun"),
        "the determinism step no longer reruns the capture"
    );
    assert!(
        joined.contains("annotate error testbed-determinism")
            && joined.contains("annotate notice testbed-determinism"),
        "the step must publish testbed-determinism as an error or a notice"
    );
    assert!(joined.contains("exit 1"), "the step never fails");
    assert!(
        code.iter().any(|line| {
            let line = line.trim_start();
            line.starts_with("if ")
                && line.contains("\"$rerun_rc\"")
                && line.contains("\"$compare_rc\"")
        }),
        "the verdict does not depend on both the rerun's and the comparison's exit code:\n{joined}"
    );
    for forbidden in ["jq", "<(", "cmp -s", "norm()"] {
        assert!(
            !joined.contains(forbidden),
            "the determinism step still holds {forbidden:?}"
        );
    }
}

/// The comment block directly above the step `name`, each line without its
/// `#`, joined with single spaces.
fn comment_above(text: &str, name: &str) -> String {
    let header = format!("- name: {name}");
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == header)
        .unwrap_or_else(|| panic!("{WORKFLOW} has no step {name:?}"));
    let mut block: Vec<&str> = Vec::new();
    for line in lines[..at].iter().rev() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            break;
        }
        block.push(trimmed.trim_start_matches('#').trim());
    }
    block.reverse();
    block.join(" ")
}

#[test]
fn build_02_determinism_comment_lists_every_exclusion() {
    // G-02-2: the comment said "the only exclusions" while the code left more out.
    // It must name every string the code constants hold.
    let text = read_repo_file(WORKFLOW);
    let comment = comment_above(&text, DETERMINISM_STEP);
    let wanted = DETERMINISM_EXCLUSIONS
        .iter()
        .chain(CONFIGURE_HASH_EXCLUSIONS.iter())
        .chain(NORMALISED_CONFIGURE_FILES.iter());
    for exclusion in wanted {
        assert!(
            comment.contains(exclusion),
            "the comment above the determinism step omits {exclusion:?}:\n{comment}"
        );
    }
}

#[test]
fn build_02_capture_notice_shows_the_normalised_configure_evidence() {
    // G-02-2: the testbed-capture notice carries the Makefile2 hash and its rule.
    let text = read_repo_file(WORKFLOW);
    let evidence = function_body(&text, "evidence");
    assert!(
        evidence.contains("normalised_configure_files"),
        "evidence() never reads normalised_configure_files"
    );
    assert!(
        evidence.contains(" normalised ") && evidence.contains("rule="),
        "evidence() does not print a `<target>/<config> normalised <key> rule=<rule> sha256=<sha256>` line"
    );
}

#[test]
fn build_02_workflow_runs_the_whole_bed() {
    // WR-08: the floor of plan 02-16 applies to every `--all` step, so the
    // evidence run must be one.
    let text = read_repo_file(WORKFLOW);
    let env: Vec<&str> = text
        .lines()
        .skip_while(|line| *line != "env:")
        .skip(1)
        .take_while(|line| line.starts_with(' '))
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .collect();
    assert_eq!(
        env,
        ["TESTBED_SELECTION: \"--all\""],
        "the top-level env must hold exactly TESTBED_SELECTION: \"--all\""
    );
}
