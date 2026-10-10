//! Static TOOL-02 and TOOL-03 checks over the real `container/pins.toml`.
//!
//! These run under plain `cargo test` with no container and no network
//! (CONTEXT D-26): the committed pins must be well formed, the provisional LLVM
//! and bitcode-rustc pins must be backed by a matching PROJECT.md decision
//! (CONTEXT D-21), `rust-toolchain.toml` must agree with the tool toolchain pin,
//! and the version each tool is expected to print must agree with the source it
//! is built from.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use mt_toolchain::check::{Observation, ToolStatus, evaluate};
use mt_toolchain::pins::{LLVM_RULE_TOOLS, LlvmSource, LlvmStatus, Pins, SourcePin, ToolPinStatus};

/// The repository root: two levels above this crate's manifest directory.
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

/// The committed container pins, parsed but not yet validated.
fn container_pins() -> Pins {
    Pins::parse(&read_repo_file("container/pins.toml")).unwrap()
}

/// The `- **D-NN:**` line of a PROJECT.md decisions block, if present.
fn decision_line<'a>(project: &'a str, id: &str) -> Option<&'a str> {
    let prefix = format!("- **{id}:**");
    project.lines().find(|line| line.starts_with(&prefix))
}

/// True when `line` carries `token` as a whole whitespace-separated word.
fn has_token(line: &str, token: &str) -> bool {
    line.split_whitespace()
        .map(|word| word.trim_end_matches([',', ';', '.', ')']))
        .any(|word| word == token)
}

/// Every way the LLVM and bitcode-rustc pins fail to match their decisions in
/// `project` (the text of PROJECT.md). Empty when the gate passes (CONTEXT D-21).
fn decision_gate_issues(pins: &Pins, project: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let checks = [
        (
            "llvm.decision",
            pins.llvm.decision.as_str(),
            format!("llvm={}", pins.llvm.major),
        ),
        (
            "rust.bitcode.decision",
            pins.rust.bitcode.decision.as_str(),
            format!("bitcode-rustc={}", pins.rust.bitcode.version),
        ),
    ];
    for (field, id, token) in checks {
        match decision_line(project, id) {
            None => issues.push(format!("{field}: PROJECT.md has no `- **{id}:**` line")),
            Some(line) if !has_token(line, &token) => {
                issues.push(format!("{field}: decision {id} does not carry `{token}`"));
            }
            Some(_) => {}
        }
    }
    issues
}

/// Sorted copy of a name list, for set comparison.
fn as_set(names: &[String]) -> BTreeSet<&str> {
    names.iter().map(String::as_str).collect()
}

#[test]
fn tool_03_container_pins_are_well_formed() {
    let pins = container_pins();
    pins.validate().unwrap();
}

#[test]
fn tool_02_container_pins_llvm_major_in_range_and_provisional() {
    let pins = container_pins();
    assert!(
        (16..=19).contains(&pins.llvm.major),
        "llvm.major {} is outside 16..=19",
        pins.llvm.major
    );
    assert_eq!(pins.llvm.max_major, 19);
    assert_eq!(
        pins.llvm.status,
        LlvmStatus::Provisional,
        "the pin stays provisional until the Phase 6 R1 verdict"
    );
}

#[test]
fn tool_02_llvm_and_bitcode_pins_have_matching_project_decision() {
    let pins = container_pins();
    let project = read_repo_file(".planning/PROJECT.md");
    assert_eq!(
        decision_gate_issues(&pins, &project),
        Vec::<String>::new(),
        "the pins and the PROJECT.md decision lines disagree"
    );

    // Changing the bitcode rustc without a new decision must fail the gate.
    let mut moved = pins.clone();
    moved.rust.bitcode.version = "1.73.0".to_owned();
    let issues = decision_gate_issues(&moved, &project);
    assert!(
        issues.iter().any(|i| i.contains("bitcode-rustc=1.73.0")),
        "bitcode 1.73.0 must fail the gate, got {issues:?}"
    );

    // So must changing the LLVM major.
    let mut moved = pins.clone();
    moved.llvm.major = 17;
    let issues = decision_gate_issues(&moved, &project);
    assert!(
        issues.iter().any(|i| i.contains("llvm=17")),
        "llvm 17 must fail the gate, got {issues:?}"
    );

    // A decision ID with no line in PROJECT.md fails, and so does an empty file.
    let mut unknown = pins.clone();
    unknown.llvm.decision = "D-99".to_owned();
    assert!(!decision_gate_issues(&unknown, &project).is_empty());
    assert!(!decision_gate_issues(&pins, "").is_empty());
}

#[test]
fn tool_03_rust_toolchain_file_matches_pins() {
    let pins = container_pins();
    let text = read_repo_file("rust-toolchain.toml");
    let file: toml::Table = toml::from_str(&text).unwrap();
    let toolchain = file["toolchain"].as_table().unwrap();
    let strings = |key: &str| -> Vec<String> {
        toolchain[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(
        toolchain["channel"].as_str().unwrap(),
        pins.rust.tool.version,
        "rust-toolchain.toml channel must equal [rust.tool].version"
    );
    assert_eq!(
        as_set(&strings("components")),
        as_set(&pins.rust.tool.components),
        "rust-toolchain.toml components must equal [rust.tool].components"
    );
    assert_eq!(
        as_set(&strings("targets")),
        as_set(&pins.rust.tool.targets),
        "rust-toolchain.toml targets must equal [rust.tool].targets"
    );
}

#[test]
fn tool_03_tool_expectations_agree_with_sources() {
    let pins = container_pins();
    let expect = |tool: &str| -> &str { &pins.tool[tool].expect };

    let SourcePin::Git { tag, .. } = &pins.source["klee"] else {
        panic!("source.klee must be a git source");
    };
    assert_eq!(
        tag.strip_prefix('v'),
        Some(expect("klee")),
        "klee expect must equal the source tag without its leading v"
    );

    for (tool, source) in [("c2rust", "c2rust"), ("cargo_mutants", "cargo_mutants")] {
        let SourcePin::Crate { version, .. } = &pins.source[source] else {
            panic!("source.{source} must be a crate source");
        };
        assert_eq!(expect(tool), version, "{tool} expect must equal its crate");
    }

    let SourcePin::Archive { version, .. } = &pins.source["arm_gnu"] else {
        panic!("source.arm_gnu must be an archive source");
    };
    assert_eq!(
        expect("arm_gcc").to_lowercase(),
        version.to_lowercase(),
        "arm_gcc expect must equal the Arm archive version"
    );

    let SourcePin::Archive { version, .. } = &pins.source["rustup"] else {
        panic!("source.rustup must be an archive source");
    };
    assert_eq!(
        expect("rustup"),
        version,
        "rustup expect must equal rustup-init"
    );

    // Tools installed from apt: the expected version follows the pinned package.
    let apt = |package: &str| -> &str { &pins.apt[package] };
    for (tool, package) in [
        ("clang", "clang-16"),
        ("llvm_config", "llvm-16"),
        ("bear", "bear"),
    ] {
        assert_eq!(
            expect(tool),
            mt_toolchain::version::upstream_version(apt(package)),
            "{tool} expect must be the upstream part of the {package} pin"
        );
    }
    for (tool, package) in [
        ("qemu_system_arm", "qemu-system-arm"),
        ("qemu_arm", "qemu-user"),
    ] {
        assert_eq!(
            expect(tool),
            apt(package),
            "{tool} expect must be the full Debian version of the {package} pin"
        );
    }

    for (key, spec) in &pins.tool {
        if spec.status != ToolPinStatus::Installed {
            continue;
        }
        if let Some(value) = spec.env.get("RUSTUP_HOME") {
            assert_eq!(value, &pins.rust.rustup_home, "tool.{key}.env RUSTUP_HOME");
        }
        if let Some(value) = spec.env.get("CARGO_HOME") {
            assert_eq!(value, &pins.rust.cargo_home, "tool.{key}.env CARGO_HOME");
        }
    }
}

// ---------------------------------------------------------------------------
// Dockerfile and workflow lints (TOOL-03): nothing floats, nothing is unchecked.
// ---------------------------------------------------------------------------

/// Hosts whose package or key material is not fixed by a snapshot timestamp.
const FLOATING_APT_HOSTS: &[&str] = &["deb.debian.org", "security.debian.org", "apt.llvm.org"];

/// Dockerfile instructions as `(KEYWORD, arguments)`: comment lines dropped and
/// continuation lines joined, the way the Dockerfile parser reads them.
fn dockerfile_instructions(text: &str) -> Vec<(String, String)> {
    let mut joined = Vec::new();
    let mut current = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(head) = line.strip_suffix('\\') {
            current.push_str(head);
            current.push(' ');
            continue;
        }
        current.push_str(line);
        joined.push(std::mem::take(&mut current));
    }
    if !current.is_empty() {
        joined.push(current);
    }
    joined
        .into_iter()
        .map(
            |instruction| match instruction.split_once(char::is_whitespace) {
                Some((keyword, rest)) => (keyword.to_uppercase(), rest.trim().to_owned()),
                None => (instruction.to_uppercase(), String::new()),
            },
        )
        .collect()
}

/// The command segments of a `RUN`: the text split on `;` and `&&`, trimmed,
/// with empty pieces dropped. A segment is the unit one `rustup` command lives in.
fn run_segments(run: &str) -> Vec<&str> {
    run.split("&&")
        .flat_map(|part| part.split(';'))
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// The checks that apply to one `RUN` instruction.
fn run_issues(run: &str, issues: &mut Vec<String>) {
    let curl = regex::Regex::new(r"(?:^|[\s;&|(])curl\s+-").unwrap();
    let pipe_to_shell = regex::Regex::new(r"\|\s*(?:sudo\s+)?(?:ba|da|z)?sh\b").unwrap();
    let pinned_version = regex::Regex::new(r#"--version\s+"?="#).unwrap();
    let dist_server =
        regex::Regex::new(r#"RUSTUP_DIST_SERVER=("[^"]*"|'[^']*'|[^\s;&|]*)"#).unwrap();
    if pipe_to_shell.is_match(run) {
        issues.push(format!("RUN pipes into a shell: {run}"));
    }
    if curl.is_match(run) && !run.contains("sha256sum -c") {
        issues.push(format!("RUN uses curl without `sha256sum -c`: {run}"));
    }
    if run.contains("git clone") && !run.contains("rev-parse HEAD") {
        issues.push(format!(
            "RUN clones without checking `rev-parse HEAD`: {run}"
        ));
    }
    if run.contains("cargo install") && !(run.contains("--locked") && pinned_version.is_match(run))
    {
        issues.push(format!(
            "RUN cargo install lacks `--locked` or an exact `--version =`: {run}"
        ));
    }

    // CR-01: a toolchain is installed only from a local mirror of files that were
    // checked in this RUN, and never lets rustup replace the pinned rustup binary.
    let servers: Vec<&str> = dist_server
        .captures_iter(run)
        .map(|captures| captures.get(1).map_or("", |m| m.as_str()))
        .map(|value| value.trim_matches(['"', '\'']))
        .collect();
    for value in &servers {
        if !value.starts_with("file://") {
            issues.push(format!(
                "RUN sets RUSTUP_DIST_SERVER to `{value}`, whose scheme is not file: {run}"
            ));
        }
    }
    if run.contains("rustup toolchain install") {
        if !servers.iter().any(|value| value.starts_with("file://")) {
            issues.push(format!(
                "RUN installs a toolchain without RUSTUP_DIST_SERVER=file://..., so rustup fetches its own copy of what was checked: {run}"
            ));
        }
        for segment in run_segments(run) {
            if segment.contains("rustup toolchain install") && !segment.contains("--no-self-update")
            {
                issues.push(format!(
                    "rustup toolchain install without `--no-self-update` may replace the pinned rustup: {segment}"
                ));
            }
        }
    }
}

/// Every way `text` lets something float: tags, frontends, unchecked downloads
/// and apt sources outside the snapshot archive.
fn dockerfile_issues(text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    let syntax = regex::Regex::new(r"(?im)^\s*#\s*syntax\s*=").unwrap();
    if syntax.is_match(text) {
        issues.push("a syntax directive pulls a floating frontend".to_owned());
    }
    let apt_source = regex::Regex::new(r"\bdeb\s+(?:\[[^\]]*\]\s+)?https?://\S+").unwrap();
    let mut stages: Vec<String> = Vec::new();
    for (keyword, rest) in dockerfile_instructions(text) {
        if rest.contains(":latest") {
            issues.push(format!("{keyword} uses :latest: {rest}"));
        }
        for host in FLOATING_APT_HOSTS {
            if rest.contains(host) {
                issues.push(format!("{keyword} names the floating host {host}"));
            }
        }
        for source in apt_source.find_iter(&rest) {
            if !source.as_str().contains("snapshot.debian.org") {
                issues.push(format!(
                    "apt source outside snapshot.debian.org: {}",
                    source.as_str()
                ));
            }
        }
        match keyword.as_str() {
            "FROM" => {
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                let flags = tokens.iter().take_while(|t| t.starts_with("--")).count();
                let image = tokens.get(flags).copied();
                let platform = tokens[..flags]
                    .iter()
                    .find_map(|t| t.strip_prefix("--platform="));
                match image {
                    None => issues.push("FROM without an image".to_owned()),
                    Some(name) if stages.iter().any(|s| s == name) => {}
                    Some(name) => {
                        if name != "${BASE_REF}@${BASE_DIGEST}" || platform != Some("${PLATFORM}") {
                            issues.push(format!(
                                "FROM {rest}: an external image must be `--platform=${{PLATFORM}} ${{BASE_REF}}@${{BASE_DIGEST}}`"
                            ));
                        }
                    }
                }
                let alias = tokens
                    .get(flags + 1)
                    .filter(|t| t.eq_ignore_ascii_case("as"))
                    .and_then(|_| tokens.get(flags + 2));
                if let Some(alias) = alias {
                    stages.push((*alias).to_owned());
                }
            }
            "RUN" => run_issues(&rest, &mut issues),
            "ENV" => {
                for name in ["RUSTUP_DIST_SERVER", "RUSTUP_UPDATE_ROOT"] {
                    if rest.contains(name) {
                        issues.push(format!(
                            "ENV sets {name}; the mirror must stay local to the RUN that installs from it: {rest}"
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    issues
}

/// `(global ARG names, problems)`: the ARGs declared before the first `FROM`, and
/// every `ARG` anywhere in the file that carries a default value.
fn dockerfile_args(text: &str) -> (Vec<String>, Vec<String>) {
    let mut global = Vec::new();
    let mut problems = Vec::new();
    let mut seen_from = false;
    for (keyword, rest) in dockerfile_instructions(text) {
        match keyword.as_str() {
            "FROM" => seen_from = true,
            "ARG" => {
                if rest.contains('=') {
                    problems.push(format!("ARG {rest}: build arguments must have no default"));
                }
                let name = rest.split('=').next().unwrap_or_default().trim().to_owned();
                if !seen_from {
                    global.push(name);
                }
            }
            _ => {}
        }
    }
    (global, problems)
}

/// Problems with the `uses:` values of one workflow file.
fn workflow_use_issues(file: &str, text: &str) -> (usize, Vec<String>) {
    let line = regex::Regex::new(r"^\s*(?:-\s+)?uses:\s*(\S+)").unwrap();
    let pinned =
        regex::Regex::new(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_./-]+)?@[0-9a-f]{40}$")
            .unwrap();
    let mut count = 0;
    let mut issues = Vec::new();
    for raw in text.lines() {
        let Some(captures) = line.captures(raw) else {
            continue;
        };
        count += 1;
        let value = captures[1].trim_matches(['"', '\'']);
        if value.starts_with("docker://") {
            issues.push(format!("{file}: docker:// reference {value}"));
        } else if !pinned.is_match(value) {
            issues.push(format!(
                "{file}: {value} is not pinned to a 40-hex commit SHA"
            ));
        }
    }
    (count, issues)
}

#[test]
fn tool_03_dockerfile_has_no_floating_references() {
    let dockerfile = read_repo_file("container/Dockerfile");
    assert_eq!(dockerfile_issues(&dockerfile), Vec::<String>::new());

    // What must be refused.
    let digest = format!("sha256:{}", "a".repeat(64));
    let refused = [
        ("syntax directive", "# syntax=docker/dockerfile:1\nFROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\n".to_owned()),
        ("floating tag", "FROM --platform=${PLATFORM} debian:latest AS a\n".to_owned()),
        ("digest without platform", format!("FROM debian@{digest} AS a\n")),
        ("unnamed stage", "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\nFROM b\n".to_owned()),
        ("curl without checksum", "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\nRUN curl -fsSL https://example.org/x -o /x\n".to_owned()),
        ("pipe into sh", "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\nRUN echo hi | sh\n".to_owned()),
        ("clone without commit check", "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\nRUN git clone --depth 1 https://example.org/r /r\n".to_owned()),
        ("install without --locked", "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\nRUN cargo install foo --version =1.0.0\n".to_owned()),
        ("install without exact version", "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\nRUN cargo install --locked foo\n".to_owned()),
        ("apt source off snapshot", "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\nRUN echo \"deb http://deb.debian.org/debian bookworm main\" > /etc/apt/sources.list\n".to_owned()),
    ];
    for (name, text) in refused {
        assert!(
            !dockerfile_issues(&text).is_empty(),
            "{name} must be refused"
        );
    }

    // What must be accepted: a `curl` package name is not a download, and a
    // checked download, a checked clone and a pinned install are fine.
    let accepted = "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\n\
        FROM a AS b\n\
        RUN apt-get install -y curl git\n\
        RUN curl -fsSLO https://example.org/x \\\n && echo \"h  x\" | sha256sum -c -\n\
        RUN git clone --branch t https://example.org/r /r \\\n && test \"$(git -C /r rev-parse HEAD)\" = c\n\
        RUN cargo install --locked foo --version =1.0.0\n\
        RUN echo \"deb [check-valid-until=no] http://snapshot.debian.org/archive/debian/x/ s main\"\n";
    assert_eq!(dockerfile_issues(accepted), Vec::<String>::new());
}

/// The digest-pinned first line every fixture Dockerfile starts with.
const FIXTURE_FROM: &str = "FROM --platform=${PLATFORM} ${BASE_REF}@${BASE_DIGEST} AS a\n";

/// `RUN` arguments of the rustup stage as the Dockerfile had them before CR-01 was
/// fixed, copied verbatim: the channel manifests are downloaded and checked, and
/// then `rustup toolchain install` fetches its own copies.
const CR01_RUSTUP_RUN: &str = r##"set -eux; \
    : "${RUST_TOOL_VERSION:?}" "${RUST_TOOL_CHANNEL_SHA256:?}" "${RUST_BITCODE_VERSION:?}" "${RUST_BITCODE_CHANNEL_SHA256:?}"; \
    : "${RUST_TOOL_COMPONENTS:?}" "${RUST_TOOL_TARGETS:?}"; \
    for pair in "${RUST_TOOL_VERSION}:${RUST_TOOL_CHANNEL_SHA256}" "${RUST_BITCODE_VERSION}:${RUST_BITCODE_CHANNEL_SHA256}"; do \
      version="${pair%%:*}"; \
      sha="${pair#*:}"; \
      curl -fsSL --retry 5 --retry-delay 5 -o "/tmp/channel-rust-${version}.toml" "https://static.rust-lang.org/dist/channel-rust-${version}.toml"; \
      echo "${sha}  /tmp/channel-rust-${version}.toml" | sha256sum -c -; \
    done; \
    extra=""; \
    for component in $(printf '%s' "${RUST_TOOL_COMPONENTS}" | tr ',' ' '); do extra="${extra} -c ${component}"; done; \
    for target in $(printf '%s' "${RUST_TOOL_TARGETS}" | tr ',' ' '); do extra="${extra} -t ${target}"; done; \
    rustup toolchain install "${RUST_TOOL_VERSION}" --profile minimal ${extra}; \
    rustup toolchain install "${RUST_BITCODE_VERSION}" --profile minimal; \
    rustup default "${RUST_TOOL_VERSION}"; \
    rustup toolchain list; \
    rm -f /tmp/channel-rust-*.toml; \
    chmod -R a+rX "${RUSTUP_HOME}" "${CARGO_HOME}""##;

/// A fixture Dockerfile holding one `RUN` of `run`.
fn run_fixture(run: &str) -> String {
    format!("{FIXTURE_FROM}RUN {run}\n")
}

/// True when some issue in `issues` mentions `needle`.
fn mentions(issues: &[String], needle: &str) -> bool {
    issues.iter().any(|issue| issue.contains(needle))
}

#[test]
fn tool_03_dockerfile_installs_rust_toolchains_from_verified_manifests() {
    // What must be refused: the pre-fix RUN verbatim (a checked download that
    // rustup then ignores), an install without a local mirror, one without
    // `--no-self-update` anywhere or in only one of two installs, and a dist server
    // that is not a file:// mirror, as a command prefix or as ENV.
    let issues = dockerfile_issues(&run_fixture(CR01_RUSTUP_RUN));
    assert!(
        mentions(&issues, "without RUSTUP_DIST_SERVER=file://"),
        "the pre-fix rustup RUN must be refused, got {issues:?}"
    );
    assert!(
        mentions(&issues, "--no-self-update"),
        "the pre-fix rustup RUN installs without --no-self-update, got {issues:?}"
    );

    let check = "curl -fsSL -o /m/dist/channel-rust-1.0.0.toml https://static.rust-lang.org/dist/channel-rust-1.0.0.toml \
        && echo \"h  /m/dist/channel-rust-1.0.0.toml\" | sha256sum -c -";
    let no_self_update = dockerfile_issues(&run_fixture(&format!(
        "{check} && RUSTUP_DIST_SERVER=\"file:///m\" rustup toolchain install 1.0.0 --profile minimal"
    )));
    assert!(
        mentions(&no_self_update, "--no-self-update"),
        "a mirror install without --no-self-update must be refused, got {no_self_update:?}"
    );
    let only_first = dockerfile_issues(&run_fixture(&format!(
        "{check} && RUSTUP_DIST_SERVER=\"file:///m\" rustup toolchain install 1.0.0 --profile minimal --no-self-update; \
         RUSTUP_DIST_SERVER=\"file:///m\" rustup toolchain install 2.0.0 --profile minimal"
    )));
    assert!(
        mentions(&only_first, "rustup toolchain install 2.0.0"),
        "the second install lacks --no-self-update and must be named, got {only_first:?}"
    );
    assert!(
        !mentions(&only_first, "rustup toolchain install 1.0.0"),
        "the first install has --no-self-update and must not be named, got {only_first:?}"
    );
    let https = dockerfile_issues(&run_fixture(&format!(
        "{check} && RUSTUP_DIST_SERVER=https://static.rust-lang.org rustup toolchain install 1.0.0 --profile minimal --no-self-update"
    )));
    assert!(
        mentions(&https, "scheme is not file"),
        "an https dist server must be refused, got {https:?}"
    );
    assert!(
        mentions(&https, "without RUSTUP_DIST_SERVER=file://"),
        "an https dist server is not a mirror, got {https:?}"
    );
    let env = dockerfile_issues(&format!("{FIXTURE_FROM}ENV RUSTUP_DIST_SERVER=file:///m\n"));
    assert!(
        mentions(&env, "ENV sets RUSTUP_DIST_SERVER"),
        "an ENV dist server must be refused, got {env:?}"
    );
    let update_root =
        dockerfile_issues(&format!("{FIXTURE_FROM}ENV RUSTUP_UPDATE_ROOT=file:///m\n"));
    assert!(
        mentions(&update_root, "ENV sets RUSTUP_UPDATE_ROOT"),
        "an ENV update root must be refused, got {update_root:?}"
    );

    // What must be accepted: a manifest downloaded into the mirror, checked, and
    // installed from through a RUN-local file:// dist server without self-update.
    let accepted = dockerfile_issues(&run_fixture(&format!(
        "mkdir -p /m/dist && {check} && RUSTUP_DIST_SERVER=\"file:///m\" rustup toolchain install 1.0.0 --profile minimal --no-self-update"
    )));
    assert_eq!(accepted, Vec::<String>::new());

    // The real Dockerfile passes, and the check is not vacuous.
    let dockerfile = read_repo_file("container/Dockerfile");
    assert_eq!(dockerfile_issues(&dockerfile), Vec::<String>::new());
    let installing = dockerfile_instructions(&dockerfile)
        .iter()
        .filter(|(keyword, rest)| keyword == "RUN" && rest.contains("rustup toolchain install"))
        .count();
    assert!(
        installing >= 1,
        "no RUN installs a Rust toolchain; the check would be vacuous"
    );
}

#[test]
fn tool_03_dockerfile_args_match_pins() {
    let pins = container_pins();
    let rendered =
        mt_toolchain::build_args::render(&pins, mt_toolchain::build_args::Scope::Image).unwrap();
    let want: BTreeSet<String> = rendered.keys().cloned().collect();

    let (global, problems) = dockerfile_args(&read_repo_file("container/Dockerfile"));
    assert_eq!(problems, Vec::<String>::new());
    let got: BTreeSet<String> = global.iter().cloned().collect();
    assert_eq!(
        global.len(),
        got.len(),
        "a global ARG is declared twice: {global:?}"
    );
    assert_eq!(
        got.difference(&want).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "the Dockerfile declares ARGs that `mt toolchain build-args` does not render"
    );
    assert_eq!(
        want.difference(&got).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "`mt toolchain build-args` renders keys the Dockerfile does not declare"
    );

    // A default value is refused wherever it appears.
    let (_, problems) = dockerfile_args("ARG A=1\nFROM x\nARG B=2\n");
    assert_eq!(problems.len(), 2);
}

#[test]
fn tool_03_workflows_pin_actions_by_commit_sha() {
    let dir = repo_root().join(".github/workflows");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no workflow files under .github/workflows"
    );

    let mut uses = 0;
    let mut issues = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let (count, found) = workflow_use_issues(&name, &fs::read_to_string(path).unwrap());
        uses += count;
        issues.extend(found);
    }
    assert!(uses > 0, "no `uses:` found; the check would be vacuous");
    assert_eq!(issues, Vec::<String>::new());

    // What must be refused and accepted.
    let sha = "3d3c42e5aac5ba805825da76410c181273ba90b1";
    for bad in [
        "uses: actions/checkout@v4".to_owned(),
        format!("uses: actions/checkout@{}", sha.to_uppercase()),
        format!("uses: actions/checkout@{}", &sha[..39]),
        format!("- uses: docker://alpine@sha256:{}", "a".repeat(64)),
        "uses: ./local-action".to_owned(),
    ] {
        assert!(
            !workflow_use_issues("t.yml", &bad).1.is_empty(),
            "{bad} must be refused"
        );
    }
    let good = format!("- uses: actions/checkout@{sha} # v7.0.1\n  uses: owner/repo/sub@{sha}\n");
    assert_eq!(workflow_use_issues("t.yml", &good), (2, Vec::new()));
}

#[test]
fn tool_03_container_pins_are_complete() {
    let pins = container_pins();
    pins.validate_complete().unwrap();
}

#[test]
fn tool_03_discovery_package_list_matches_apt_pins() {
    let pins = container_pins();
    let workflow = read_repo_file(".github/workflows/pin-discovery.yml");
    let list = regex::Regex::new(r#"(?m)^\s*packages="([^"]+)"\s*$"#).unwrap();
    let captures = list
        .captures(&workflow)
        .expect("pin-discovery.yml must define a `packages=\"...\"` line");
    let major = pins.llvm.major.to_string();
    let discovered: BTreeSet<String> = captures[1]
        .split_whitespace()
        .map(|name| name.replace("${LLVM_MAJOR}", &major))
        .collect();
    let pinned: BTreeSet<String> = pins.apt.keys().cloned().collect();
    assert_eq!(
        discovered, pinned,
        "the packages pin-discovery.yml resolves must be exactly the [apt] keys"
    );
}

// ---------------------------------------------------------------------------
// Image stages (plan 01-07): the apt set, LLVM literals and COPY sources.
// ---------------------------------------------------------------------------

/// The `RUN` instructions of stage `stage` that install packages with `apt-get`.
fn stage_apt_installs(text: &str, stage: &str) -> Vec<String> {
    let mut current = String::new();
    let mut found = Vec::new();
    for (keyword, rest) in dockerfile_instructions(text) {
        match keyword.as_str() {
            "FROM" => {
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                current = tokens
                    .iter()
                    .position(|t| t.eq_ignore_ascii_case("as"))
                    .and_then(|at| tokens.get(at + 1))
                    .map(|name| (*name).to_owned())
                    .unwrap_or_default();
            }
            "RUN" if current == stage && rest.contains("apt-get install") => found.push(rest),
            _ => {}
        }
    }
    found
}

/// The pinned apt packages that the `apt-base` install command does not name, with
/// `${LLVM_MAJOR}` replaced by `major` the way the build argument does.
fn apt_install_gaps(dockerfile: &str, apt_keys: &[String], major: u32) -> Vec<String> {
    let major = major.to_string();
    let installs = stage_apt_installs(dockerfile, "apt-base");
    let named: BTreeSet<String> = installs
        .iter()
        .flat_map(|run| run.split_whitespace())
        .map(|word| word.replace("${LLVM_MAJOR}", &major))
        .map(|word| word.trim_end_matches(';').to_owned())
        .collect();
    apt_keys
        .iter()
        .filter(|key| !named.contains(key.as_str()))
        .cloned()
        .collect()
}

/// Every `llvm-N`, `clang-N` or `clang-cppN` literal in `text`.
fn llvm_major_literals(text: &str) -> Vec<String> {
    let literal = regex::Regex::new(r"(?:llvm-|clang-cpp|clang-)\d+").unwrap();
    literal
        .find_iter(text)
        .map(|found| found.as_str().to_owned())
        .collect()
}

/// Every `COPY`/`ADD` that does not copy from an earlier stage.
fn copy_issues(text: &str) -> Vec<String> {
    let mut stages: Vec<String> = Vec::new();
    let mut issues = Vec::new();
    for (keyword, rest) in dockerfile_instructions(text) {
        match keyword.as_str() {
            "FROM" => {
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                if let Some(at) = tokens.iter().position(|t| t.eq_ignore_ascii_case("as"))
                    && let Some(name) = tokens.get(at + 1)
                {
                    stages.push((*name).to_owned());
                }
            }
            "ADD" => issues.push(format!("ADD is not allowed: {rest}")),
            "COPY" => {
                let from = rest
                    .split_whitespace()
                    .find_map(|word| word.strip_prefix("--from="));
                match from {
                    Some(name) if stages.iter().any(|s| s == name) => {}
                    Some(name) => issues.push(format!(
                        "COPY --from={name} is not an earlier stage of this file: {rest}"
                    )),
                    None => issues.push(format!("COPY without --from reads the context: {rest}")),
                }
            }
            _ => {}
        }
    }
    issues
}

#[test]
fn tool_03_dockerfile_installs_every_apt_pin() {
    let pins = container_pins();
    let keys: Vec<String> = pins.apt.keys().cloned().collect();
    assert!(
        !keys.is_empty(),
        "no [apt] pins; the check would be vacuous"
    );
    let dockerfile = read_repo_file("container/Dockerfile");
    assert!(
        !stage_apt_installs(&dockerfile, "apt-base").is_empty(),
        "stage apt-base has no `apt-get install`"
    );
    assert_eq!(
        apt_install_gaps(&dockerfile, &keys, pins.llvm.major),
        Vec::<String>::new(),
        "every [apt] key must be named in the apt-base install command"
    );

    // A package left out of the install command is reported, and so is a
    // Dockerfile with no apt-base stage at all.
    let missing = "FROM x AS apt-base\nRUN apt-get install --yes clang-${LLVM_MAJOR} bear\n";
    let want = vec![
        "clang-16".to_owned(),
        "llvm-16".to_owned(),
        "bear".to_owned(),
    ];
    assert_eq!(apt_install_gaps(missing, &want, 16), vec!["llvm-16"]);
    assert_eq!(apt_install_gaps("FROM x AS other\n", &want, 16), want);
    // Installing in another stage does not count.
    let elsewhere = "FROM x AS build-base\nRUN apt-get install --yes bear\n";
    assert_eq!(
        apt_install_gaps(elsewhere, &["bear".to_owned()], 16),
        vec!["bear"]
    );
}

#[test]
fn tool_02_dockerfile_has_no_llvm_major_literal() {
    let dockerfile = read_repo_file("container/Dockerfile");
    assert_eq!(
        llvm_major_literals(&dockerfile),
        Vec::<String>::new(),
        "the LLVM major reaches the Dockerfile only as ${{LLVM_MAJOR}}"
    );
    assert!(
        dockerfile.contains("${LLVM_MAJOR}"),
        "the Dockerfile must use ${{LLVM_MAJOR}}; the check would be vacuous"
    );

    // What must be refused and accepted.
    for bad in [
        "apt-get install clang-16",
        "/usr/lib/llvm-16/bin",
        "libclang-cpp16-dev",
        "# llvm-18",
    ] {
        assert!(
            !llvm_major_literals(bad).is_empty(),
            "{bad} must be refused"
        );
    }
    for good in [
        "clang-${LLVM_MAJOR}",
        "/usr/lib/llvm-${LLVM_MAJOR}/bin",
        "libclang-cpp${LLVM_MAJOR}-dev",
        "llvm-link",
    ] {
        assert!(llvm_major_literals(good).is_empty(), "{good} must pass");
    }
}

#[test]
fn tool_03_dockerfile_copies_only_between_stages() {
    let dockerfile = read_repo_file("container/Dockerfile");
    assert_eq!(copy_issues(&dockerfile), Vec::<String>::new());
    assert!(
        dockerfile_instructions(&dockerfile)
            .iter()
            .any(|(keyword, _)| keyword == "COPY"),
        "the Dockerfile has no COPY; the check would be vacuous"
    );

    // What must be refused and accepted.
    let stage = "FROM a AS one\nFROM one AS two\n";
    for bad in [
        "COPY file /file\n",
        "ADD file /file\n",
        "ADD --from=one /a /b\n",
        "COPY --from=later /a /b\n",
        "COPY --from=debian:12 /a /b\n",
    ] {
        assert!(
            !copy_issues(&format!("{stage}{bad}")).is_empty(),
            "{bad} must be refused"
        );
    }
    assert!(copy_issues(&format!("{stage}COPY --from=one /a /b\n")).is_empty());
    // A stage defined after the COPY is not "earlier".
    assert!(!copy_issues("FROM a AS one\nCOPY --from=two /a /b\nFROM a AS two\n").is_empty());
}

// ---------------------------------------------------------------------------
// Real outputs: the container pins against what the CI image printed.
// ---------------------------------------------------------------------------

/// One recorded command in `fixtures/tool-outputs.toml`.
#[derive(serde::Deserialize)]
struct CapturedOutput {
    source: String,
    stdout: String,
    stderr: String,
}

/// The captured fixtures as observations, with each LLVM probe's output attached
/// to the tool it belongs to (the probe key is the tool key plus the probe
/// program's file name, `c2rust_ldd`).
fn captured_observations(pins: &Pins) -> BTreeMap<String, Observation> {
    let text = read_repo_file("crates/mt-toolchain/tests/fixtures/tool-outputs.toml");
    let captured: BTreeMap<String, CapturedOutput> = toml::from_str(&text).unwrap();
    let mut observations: BTreeMap<String, Observation> = captured
        .iter()
        .map(|(key, record)| {
            (
                key.clone(),
                Observation::ran(key, &record.stdout, &record.stderr),
            )
        })
        .collect();
    for (key, spec) in &pins.tool {
        if spec.status != ToolPinStatus::Installed {
            continue;
        }
        let Some(rule) = &spec.llvm else { continue };
        if rule.source != LlvmSource::Probe {
            continue;
        }
        let program = Path::new(&rule.argv[0])
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap();
        let probe_key = format!("{key}_{program}");
        let probe = observations
            .remove(&probe_key)
            .unwrap_or_else(|| panic!("no captured output {probe_key} for the {key} LLVM probe"));
        let tool = observations
            .remove(key)
            .unwrap_or_else(|| panic!("no captured output for {key}"));
        observations.insert(key.clone(), tool.with_probe(probe));
    }
    observations
}

#[test]
fn tool_02_container_pins_regexes_parse_captured_outputs() {
    let pins = container_pins();
    let text = read_repo_file("crates/mt-toolchain/tests/fixtures/tool-outputs.toml");
    let captured: BTreeMap<String, CapturedOutput> = toml::from_str(&text).unwrap();
    assert!(!captured.is_empty(), "no captured outputs");
    for (key, record) in &captured {
        assert!(
            !record.source.contains("representative"),
            "{key}: still a representative output, not a capture"
        );
        assert!(
            !record.stdout.is_empty() || !record.stderr.is_empty(),
            "{key}: captured nothing"
        );
    }

    let observations = captured_observations(&pins);
    let report = evaluate(&pins, &observations);
    assert!(
        report.passed(),
        "the container pins do not accept the captured outputs:\n{}",
        report.render_table()
    );

    // Every installed tool yields exactly its expected version, and every LLVM
    // rule yields the pinned major.
    for (key, spec) in &pins.tool {
        if spec.status != ToolPinStatus::Installed {
            continue;
        }
        let row = report
            .rows
            .iter()
            .find(|row| &row.key == key)
            .unwrap_or_else(|| panic!("no row for tool {key}"));
        assert_eq!(row.status, ToolStatus::Ok, "{key}: {:?}", row.detail);
        assert_eq!(
            row.actual.as_deref(),
            Some(spec.expect.as_str()),
            "{key}: parsed version"
        );
        if spec.llvm.is_some() {
            assert_eq!(
                row.llvm_major,
                Some(pins.llvm.major),
                "{key}: LLVM major from the captured output"
            );
        }
    }
    for key in LLVM_RULE_TOOLS {
        assert!(
            pins.tool[key].llvm.is_some(),
            "{key} must carry an LLVM rule"
        );
    }

    // A capture of another LLVM major must be refused, so the test cannot pass
    // by ignoring the output.
    let mut other = observations.clone();
    let clang = other.get_mut("clang").unwrap();
    clang.stdout = clang.stdout.replace("16.0.6", "17.0.6");
    assert!(!evaluate(&pins, &other).passed());
}

// ---------------------------------------------------------------------------
// Publishing (plan 01-08): the README's honest claims and the push job's scope.
// ---------------------------------------------------------------------------

/// Words PROJECT D-13 reserves, plus the phrase CONTEXT D-06 rules out. They
/// live in this test only, so the README can be scanned for them.
const README_FORBIDDEN: &[&str] = &[
    "certified",
    "certify",
    "compliant",
    "compliance",
    "bit-for-bit reproducible",
];

/// The forbidden words and phrases found in `text`, compared case-insensitively.
fn forbidden_claims(text: &str) -> Vec<&'static str> {
    let lowered = text.to_lowercase();
    README_FORBIDDEN
        .iter()
        .copied()
        .filter(|word| lowered.contains(word))
        .collect()
}

#[test]
fn tool_03_container_readme_states_manifest_identical_and_makes_no_certification_claim() {
    let readme = read_repo_file("container/README.md");
    assert!(
        readme.contains("manifest-identical"),
        "the README must state the guarantee as manifest-identical builds"
    );
    assert!(
        readme.contains("provisional"),
        "the README must say the LLVM and bitcode-rustc pins are provisional"
    );
    assert_eq!(
        forbidden_claims(&readme),
        Vec::<&str>::new(),
        "the README makes a claim the project forbids"
    );

    // What must be refused, whatever the case, and what must pass.
    for bad in [
        "The image is Certified.",
        "a COMPLIANT toolchain",
        "bit-for-bit Reproducible builds",
        "we certify it",
    ] {
        assert!(!forbidden_claims(bad).is_empty(), "{bad} must be refused");
    }
    assert!(forbidden_claims("manifest-identical builds only").is_empty());
}

#[test]
fn tool_03_container_readme_documents_the_required_sections() {
    let readme = read_repo_file("container/README.md");
    let headings: Vec<&str> = readme
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect();
    for want in [
        "Pulling and running on Apple Silicon",
        "Local build fallback",
        "Bumping pins",
        "Notices",
        "Current image",
    ] {
        assert!(
            headings.contains(&want),
            "the README has no `## {want}` heading, got {headings:?}"
        );
    }
    for needle in [
        "mt toolchain hash",
        "--platform linux/amd64",
        "mt toolchain build-args",
        "Hayroll",
        "Kani",
    ] {
        assert!(
            readme.contains(needle),
            "the README never mentions {needle}"
        );
    }
}

#[test]
fn tool_03_container_readme_references_the_image_by_digest_only() {
    const IMAGE: &str = "ghcr.io/sat-wik/migrationtool-toolchain";
    let readme = read_repo_file("container/README.md");
    let mut found = 0;
    for (at, _) in readme.match_indices(IMAGE) {
        found += 1;
        assert!(
            readme[at + IMAGE.len()..].starts_with("@sha256:"),
            "a reference to the image near byte {at} is not by digest"
        );
    }
    assert!(found > 0, "the README names no image reference");

    // A recorded digest is 64 lowercase hex digits, and a tag is refused.
    let recorded = regex::Regex::new(&format!(r"{IMAGE}@sha256:([0-9a-f]+)\b")).unwrap();
    for captures in recorded.captures_iter(&readme) {
        let digest = &captures[1];
        assert_eq!(digest.len(), 64, "{digest} is not a full sha256 digest");
    }
}

/// The text of job `job` in a workflow: from its two-space-indented key to the next job.
fn workflow_job_text<'a>(workflow: &'a str, job: &str) -> &'a str {
    let key = format!("\n  {job}:\n");
    let start = workflow
        .find(&key)
        .unwrap_or_else(|| panic!("no job {job}"))
        + 1;
    let rest = &workflow[start + key.len() - 1..];
    let next = regex::Regex::new(r"(?m)^  [A-Za-z][A-Za-z0-9_-]*:\s*$").unwrap();
    let end = next
        .find(rest)
        .map_or(workflow.len(), |m| start + key.len() - 1 + m.start());
    &workflow[start..end]
}

#[test]
fn tool_03_workflow_publishes_by_digest_with_least_privilege() {
    let workflow = read_repo_file(".github/workflows/container.yml");

    // Write access to packages exists once, in image-a, and nowhere else.
    assert_eq!(
        workflow.matches("packages: write").count(),
        1,
        "`packages: write` must appear exactly once"
    );
    let image_a = workflow_job_text(&workflow, "image-a");
    assert!(
        image_a.contains("packages: write"),
        "`packages: write` must be in the image-a job"
    );
    for other in ["mt", "image-b", "compare"] {
        assert!(
            !workflow_job_text(&workflow, other).contains("packages:"),
            "job {other} must not grant any package permission"
        );
    }

    // Only the workflow's own token: no other secret, no personal access token.
    let secret = regex::Regex::new(r"secrets\.([A-Za-z0-9_]+)").unwrap();
    let used: BTreeSet<String> = secret
        .captures_iter(&workflow)
        .map(|c| c[1].to_owned())
        .collect();
    assert_eq!(used, BTreeSet::from(["GITHUB_TOKEN".to_owned()]));

    // The login is SHA-pinned (checked for every `uses:` by the SHA test),
    // happens in image-a only, never for a pull_request, and the push follows it.
    let login = image_a
        .find("docker/login-action@")
        .expect("image-a must log in with docker/login-action");
    assert_eq!(workflow.matches("docker/login-action@").count(), 1);
    let guard = "if: ${{ github.event_name != 'pull_request' }}";
    assert!(
        image_a[..login]
            .rfind(guard)
            .is_some_and(|at| login - at < 200),
        "the login step must be skipped for pull_request events"
    );
    let push = image_a
        .find("docker push")
        .expect("image-a must push the image");
    assert!(push > login, "the push must come after the login");
    let publish = image_a
        .find("id: publish")
        .expect("the push step must have the id `publish`");
    let publish_run = publish + image_a[publish..].find("run:").expect("publish has a run");
    assert!(
        image_a[publish..publish_run].contains(guard),
        "the push step must itself be skipped for pull_request events"
    );
    assert!(image_a.contains("ghcr.io/sat-wik/migrationtool-toolchain"));
    assert!(image_a.contains("{{index .RepoDigests 0}}"));
    assert!(
        !image_a[push..].contains("docker build") && !image_a[push..].contains("buildx build"),
        "the pushed image is never rebuilt for publishing"
    );

    // The digest is announced twice: when pushed, and as verified only in the
    // compare job, after manifest-diff succeeded.
    assert!(image_a.contains("annotate notice image-digest"));
    let compare = workflow_job_text(&workflow, "compare");
    let diff = compare
        .find("manifest-diff")
        .expect("compare runs manifest-diff");
    let verified = compare
        .find("annotate notice verified-image")
        .expect("compare must publish the verified-image notice");
    assert!(
        verified > diff,
        "verified-image must follow the manifest diff"
    );
    assert!(
        compare[diff..verified].contains("exit 1"),
        "a manifest difference must fail the job before verified-image is published"
    );
}

#[test]
fn tool_03_workflow_gates_on_pin_binding_evidence() {
    let workflow = read_repo_file(".github/workflows/container.yml");
    let image_a = workflow_job_text(&workflow, "image-a");

    // The gate reads the build record out of the built image and reports through
    // a notice on success and an error on mismatch.
    assert!(
        image_a.contains("pin-binding.sha256"),
        "image-a must read the build record pin-binding.sha256"
    );
    assert!(
        image_a.contains("annotate notice pin-binding"),
        "image-a must publish a pin-binding notice"
    );
    let error = image_a
        .find("annotate error pin-binding")
        .expect("image-a must publish a pin-binding error on mismatch");
    let after_error = &image_a[error..];
    let window: String = after_error.chars().take(300).collect();
    assert!(
        window.contains("exit 1"),
        "`exit 1` must follow the pin-binding error annotation within 300 characters"
    );

    // The gate comes before the push, and the publish steps carry no status
    // function, so a failed gate keeps the image out of GHCR.
    let first_gate = image_a
        .find("pin-binding")
        .expect("image-a mentions pin-binding");
    let publish = image_a
        .find("id: publish")
        .expect("the push step must have the id `publish`");
    assert!(
        first_gate < publish,
        "the pin-binding gate must come before the publish step"
    );
    let login = image_a
        .find("docker/login-action@")
        .expect("image-a logs in before it pushes");
    assert!(
        first_gate < login,
        "the pin-binding gate must come before the GHCR login"
    );
}
