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
const SUBCOMMANDS: [&str; 8] = [
    "validate",
    "fetch",
    "capture",
    "replay",
    "seeds",
    "compare",
    "pairs",
    "toolchains",
];

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
    let build = function_body(&text, "run_build");

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
    assert!(
        build.contains("--network none"),
        "run_build runs upstream build code and must have no network"
    );

    for (name, body) in [
        ("run_offline", &offline),
        ("run_networked", &networked),
        ("run_build", &build),
    ] {
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
    assert!(
        build.contains("/work/cache:ro"),
        "run_build must mount the source cache read-only"
    );

    // Every `docker run` lives in one of the four helpers: a step cannot start a
    // container with network access by writing its own command.
    let outside = without_functions(
        &text,
        &["run_offline", "run_networked", "image_tool", "run_build"],
    );
    assert!(
        !outside
            .lines()
            .any(|line| { !line.trim_start().starts_with('#') && line.contains("docker run") }),
        "docker run outside the four helpers"
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
    for subcommand in ["validate", "fetch", "capture", "replay", "compare", "pairs"] {
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
        ("run_build", &build),
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
    // `... | sort | head -n 1` under `pipefail` is a race: `head` exits after its
    // first line, and when the runner ignores SIGPIPE `sort` fails its remaining
    // writes with "write failed: Broken pipe" and exits 2 (testbed run B, step "Make
    // the two Arm fixture objects"). A one-write listing cannot race; the `--all`
    // listing of about 490 paths can. The pick must read the whole listing.
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

/// The index of the first line of `lines` that contains `needle`.
fn first_line_with(lines: &[String], needle: &str) -> Option<usize> {
    lines.iter().position(|line| line.contains(needle))
}

#[test]
fn build_02_untrusted_builds_cannot_write_the_evidence() {
    // WR-04: an upstream build runs only in run_build, one container per pair,
    // and can write nothing but a discarded scratch /work and its own run
    // directory. The capture artifacts, the reports and every other pair's
    // directories are not mounted into it at all.
    let text = read_repo_file(WORKFLOW);
    let build = function_body(&text, "run_build");
    let outside = without_functions(&text, &["run_build"]);

    // The build stage is started in run_build and nowhere else, comments included.
    assert!(
        build.contains("--stage build"),
        "run_build never runs `mt testbed capture --stage build`"
    );
    assert!(
        !outside.contains("--stage build"),
        "`--stage build` appears outside run_build"
    );

    // Every other capture invocation is the record stage, which runs no upstream
    // code; none may lack --stage.
    let capture_call = Regex::new(r"\b(run_offline|run_networked|mt testbed)\s+capture\b").unwrap();
    let outside_code = code_lines(&outside);
    let captures: Vec<&String> = outside_code
        .iter()
        .filter(|line| capture_call.is_match(line))
        .collect();
    assert!(
        captures.len() >= 2,
        "expected the record step and the determinism record, found {captures:?}"
    );
    for line in captures {
        assert!(
            line.contains("run_offline capture") && line.contains("--stage record"),
            "a capture invocation outside run_build is not `run_offline capture --stage record`: {line}"
        );
    }
    for line in code_lines(&build) {
        if capture_call.is_match(&line) {
            assert!(
                line.contains("--stage build"),
                "run_build runs a capture without --stage build: {line}"
            );
        }
    }

    // The only writable mounts of run_build: the fresh scratch /work and the
    // pair's run directory, which is mounted below it.
    let mounts = volume_mounts(&build);
    let writable: Vec<&String> = mounts.iter().filter(|m| !m.ends_with(":ro")).collect();
    assert_eq!(
        writable.len(),
        2,
        "run_build must have exactly two writable mounts: {writable:?}"
    );
    assert_eq!(writable[0], "$scratch/work:/work", "{writable:?}");
    assert!(
        writable[1].ends_with(":/work/run/$id/$cfg"),
        "the second writable mount is not the pair's run directory: {writable:?}"
    );
    for readonly in [
        "/usr/local/bin/mt:ro",
        "/opt/mt/pins.toml:ro",
        "/opt/mt/testbed:ro",
        ":/work/cache:ro",
    ] {
        assert!(
            mounts.iter().any(|m| m.ends_with(readonly)),
            "run_build has no read-only mount ending {readonly}: {mounts:?}"
        );
    }
    // WR-12 (plan 02-22): the toolchain file configure reads is written by a
    // container that runs no upstream code (`run_offline toolchains`, before the
    // build container) and mounted read-only at /opt/mt/toolchain, in the
    // read-only root file system and outside both writable mounts. The build
    // stage is told to use it.
    let toolchain_mounts: Vec<&String> = mounts
        .iter()
        .filter(|m| m.ends_with(":/opt/mt/toolchain:ro"))
        .collect();
    assert_eq!(
        toolchain_mounts,
        ["$work/toolchain/$id/$cfg:/opt/mt/toolchain:ro"],
        "run_build must mount the pair's trusted toolchain directory read-only at /opt/mt/toolchain: {mounts:?}"
    );
    let code = code_lines(&build);
    let build_line = code
        .iter()
        .find(|line| line.contains("--stage build"))
        .expect("run_build runs the build stage");
    assert!(
        build_line.contains("--toolchain-dir /opt/mt/toolchain"),
        "the build stage is not given the read-only toolchain: {build_line}"
    );
    let toolchains = first_line_with(&code, "run_offline toolchains")
        .unwrap_or_else(|| panic!("run_build never runs `run_offline toolchains`:\n{build}"));
    let container = first_line_with(&code, "docker run")
        .unwrap_or_else(|| panic!("run_build starts no container:\n{build}"));
    assert!(
        toolchains < container,
        "the toolchains step must run before the build container:\n{build}"
    );
    for mount in volume_mounts(&text) {
        assert!(
            !mount.contains("toolchain") || mount.ends_with(":ro"),
            "a toolchain path is mounted writable: {mount}"
        );
    }
    assert!(
        code.iter().any(|line| line
            .trim_start()
            .starts_with("scratch=\"$RUNNER_TEMP/scratch/")),
        "scratch is not assigned a path under $RUNNER_TEMP/scratch/:\n{build}"
    );
    for line in &code {
        assert!(
            !line.contains("/out") && !line.contains("OUT_HOST"),
            "run_build mounts or names /out: {line}"
        );
    }
    assert!(
        code.iter().any(|line| line.contains("already exists")),
        "run_build must refuse a run directory that already exists"
    );

    // The helpers that run no upstream code take their directories from the
    // variables the determinism rerun redirects.
    for name in ["run_offline", "run_networked", "image_tool"] {
        let body = function_body(&text, name);
        for variable in ["WORK_HOST", "OUT_HOST"] {
            assert!(body.contains(variable), "{name} ignores {variable}");
        }
    }
    assert!(
        function_body(&text, "run_offline").contains("CACHE_HOST"),
        "run_offline cannot mount a separate read-only cache"
    );

    // Step order: seeds, build, record, replay.
    let header = |name: &str| {
        text.find(&format!("- name: {name}"))
            .unwrap_or_else(|| panic!("{WORKFLOW} has no step {name:?}"))
    };
    assert!(
        header("Check the committed seeds") < header("Build each pair in its own container")
            && header("Build each pair in its own container") < header("Record the captures")
            && header("Record the captures") < header("Replay the captured builds"),
        "the steps are not in the order seeds, build, record, replay"
    );
    let build_step = step_code(&text, "Build each pair in its own container").join("\n");
    for needle in [
        "run_offline pairs",
        "run_build",
        "< /dev/null",
        "finish_step build",
    ] {
        assert!(
            build_step.contains(needle),
            "the build step lacks {needle:?}:\n{build_step}"
        );
    }
    assert!(
        step_code(&text, "Record the captures")
            .join("\n")
            .contains("finish_step capture"),
        "the record step no longer publishes testbed-capture"
    );

    // The determinism rerun: its own work directory, builds in run_build, the
    // record stage, and a comparison with the first capture.
    let rerun = step_code(&text, DETERMINISM_STEP);
    let work = first_line_with(&rerun, "WORK_HOST=")
        .unwrap_or_else(|| panic!("the determinism step never sets WORK_HOST:\n{rerun:?}"));
    assert!(
        rerun[work].contains("work-rerun"),
        "WORK_HOST is not the rerun work directory: {}",
        rerun[work]
    );
    assert!(
        rerun
            .iter()
            .any(|line| line.contains("CACHE_HOST=") && line.contains("/work/cache")),
        "the rerun does not read the first run's verified cache"
    );
    let first_build = first_line_with(&rerun, "run_build")
        .unwrap_or_else(|| panic!("the determinism step never runs run_build:\n{rerun:?}"));
    let record = first_line_with(&rerun, "run_offline capture")
        .unwrap_or_else(|| panic!("the determinism step has no record:\n{rerun:?}"));
    assert!(
        work < first_build && first_build < record,
        "the rerun must set WORK_HOST, then build, then record: {rerun:?}"
    );
    assert!(
        rerun[record].contains("--stage record")
            && rerun[record].contains("--out /out/capture-rerun"),
        "the rerun record is not `--stage record --out /out/capture-rerun`: {}",
        rerun[record]
    );
    assert!(
        rerun.iter().any(|line| line.contains(
            "run_offline compare --first /out/capture --second /out/capture-rerun/capture"
        )),
        "the rerun is not compared with /out/capture"
    );

    // The header comment states the trust split and the new title.
    assert!(text.contains("Trust split"), "no `Trust split` paragraph");
    assert!(
        text.contains("testbed-build"),
        "testbed-build is not listed"
    );
}

#[test]
fn build_02_readme_names_the_container_kinds() {
    let readme = read_repo_file("testbed/README.md");
    for needle in [
        "run_build",
        "--stage build",
        "--stage record",
        "--pids-limit",
    ] {
        assert!(
            readme.contains(needle),
            "testbed/README.md does not describe {needle:?}"
        );
    }
}

#[test]
fn build_02_record_steps_require_the_read_only_toolchain() {
    // WR-12 (plan 02-22): every record stage refuses a CMake pair that was not
    // built against the read-only toolchain file, so the read-only mode covers
    // every CMake pair the workflow captures: the record step and the
    // determinism rerun.
    let text = read_repo_file(WORKFLOW);
    let records: Vec<String> = code_lines(&text)
        .into_iter()
        .filter(|line| line.contains("--stage record"))
        .collect();
    assert_eq!(
        records.len(),
        2,
        "expected the record step and the determinism rerun: {records:?}"
    );
    for line in &records {
        assert!(
            line.contains("run_offline capture --stage record"),
            "a record line is not `run_offline capture --stage record`: {line}"
        );
        assert!(
            line.contains("--read-only-toolchain"),
            "a record line does not require the read-only toolchain: {line}"
        );
    }
}
