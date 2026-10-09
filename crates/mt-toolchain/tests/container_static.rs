//! Static TOOL-02 and TOOL-03 checks over the real `container/pins.toml`.
//!
//! These run under plain `cargo test` with no container and no network
//! (CONTEXT D-26): the committed pins must be well formed, the provisional LLVM
//! and bitcode-rustc pins must be backed by a matching PROJECT.md decision
//! (CONTEXT D-21), `rust-toolchain.toml` must agree with the tool toolchain pin,
//! and the version each tool is expected to print must agree with the source it
//! is built from.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mt_toolchain::pins::{LlvmStatus, Pins, SourcePin, ToolPinStatus};

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

/// The checks that apply to one `RUN` instruction.
fn run_issues(run: &str, issues: &mut Vec<String>) {
    let curl = regex::Regex::new(r"(?:^|[\s;&|(])curl\s+-").unwrap();
    let pipe_to_shell = regex::Regex::new(r"\|\s*(?:sudo\s+)?(?:ba|da|z)?sh\b").unwrap();
    let pinned_version = regex::Regex::new(r#"--version\s+"?="#).unwrap();
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
