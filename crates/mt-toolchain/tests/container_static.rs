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

/// Split a `RUN` into simple commands: on `&&`, `||`, `;`, `|`, `&`, `(`, `)`,
/// backticks and newlines, trimmed, with empty pieces dropped. Quoting is not
/// tracked, so a separator inside a quoted word only ever makes pieces smaller.
fn run_commands(run: &str) -> Vec<String> {
    run.split([';', '&', '|', '(', ')', '`', '\n'])
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .map(str::to_owned)
        .collect()
}

/// One simple command: its leading `NAME=value` assignments and its words.
struct ParsedCommand {
    /// The command as written, for messages.
    text: String,
    /// Leading environment assignments, quotes removed.
    env: Vec<(String, String)>,
    /// `words[0]` is the program as written; quotes removed.
    words: Vec<String>,
}

impl ParsedCommand {
    /// The program's file name: `/usr/bin/rustup` and `rustup` both give `rustup`.
    fn program(&self) -> &str {
        self.words
            .first()
            .map_or("", |word| word.rsplit('/').next().unwrap_or(word))
    }

    /// The words after the program.
    fn args(&self) -> &[String] {
        self.words.get(1..).unwrap_or(&[])
    }

    /// The value of `name` in the command's own leading assignments (the last wins).
    fn env_value(&self, name: &str) -> Option<&str> {
        self.env
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

fn unquote(word: &str) -> &str {
    word.trim_matches(['"', '\''])
}

/// Parse `text` as a simple command. Shell keywords, `sudo`/`env`/`command`
/// style wrappers and `sh -c '...'` are looked through, so an install is judged
/// by what it is, not by how it is spelled.
fn parse_command(text: &str) -> Option<ParsedCommand> {
    const SKIPPED: &[&str] = &[
        "do", "then", "else", "elif", "if", "while", "until", "!", "{", "time", "exec", "command",
        "sudo", "nohup", "nice",
    ];
    let assignment = regex::Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*=").unwrap();
    let mut env = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut rest = text.split_whitespace().peekable();
    while let Some(word) = rest.peek().copied() {
        let bare = unquote(word);
        if SKIPPED.contains(&bare) {
            rest.next();
        } else if bare == "env" {
            rest.next();
            while rest.peek().is_some_and(|next| next.starts_with('-')) {
                rest.next();
            }
        } else if assignment.is_match(word) {
            rest.next();
            if let Some((name, value)) = word.split_once('=') {
                env.push((name.to_owned(), unquote(value).to_owned()));
            }
        } else {
            break;
        }
    }
    words.extend(rest.map(|word| unquote(word).to_owned()));
    let first = words.first()?;
    let program = first.rsplit('/').next().unwrap_or(first);
    if matches!(program, "sh" | "bash" | "dash" | "zsh") {
        let position = words.iter().position(|word| {
            word.starts_with('-') && !word.starts_with("--") && word.ends_with('c')
        });
        if let Some(position) = position {
            // `sh -c "rustup ..."`: judge the inner command as written.
            let inner = words[position + 1..].join(" ");
            return parse_command(&inner).map(|mut inner| {
                inner.text = text.to_owned();
                inner
            });
        }
    }
    if matches!(program, "sh" | "bash" | "dash" | "zsh") {
        // `sh script args`: the script is the program that runs.
        if let Some(position) = words.iter().skip(1).position(|word| !word.starts_with('-')) {
            words.drain(..=position);
        }
    }
    Some(ParsedCommand {
        text: text.to_owned(),
        env,
        words,
    })
}

/// `tar` extraction arguments: the directory (`-C`) and the archive (`-f`).
fn tar_extraction(command: &ParsedCommand) -> Option<(Option<String>, Option<String>)> {
    if command.program() != "tar" {
        return None;
    }
    let mut extract = false;
    let mut directory = None;
    let mut archive = None;
    let mut pending: std::collections::VecDeque<char> = std::collections::VecDeque::new();
    for (index, arg) in command.args().iter().enumerate() {
        if arg == "--extract" || arg == "--get" {
            extract = true;
        } else if arg == "--directory" || arg == "--file" {
            pending.push_back(if arg == "--directory" { 'C' } else { 'f' });
        } else if let Some(value) = arg.strip_prefix("--directory=") {
            directory = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--file=") {
            archive = Some(value.to_owned());
        } else if arg.starts_with("--") {
            // Some other long option; it takes no value that matters here.
        } else if arg.starts_with('-') || index == 0 {
            // A cluster of short options such as `-xzf`; `C` and `f` each take
            // the next free argument, in the order they appear.
            for flag in arg.trim_start_matches('-').chars() {
                match flag {
                    'x' => extract = true,
                    'C' | 'f' => pending.push_back(flag),
                    _ => {}
                }
            }
        } else if let Some(flag) = pending.pop_front() {
            if flag == 'C' {
                directory = Some(arg.clone());
            } else {
                archive = Some(arg.clone());
            }
        }
    }
    extract.then_some((directory, archive))
}

/// True when `text` is a `sha256sum -c` check, and the words it mentions.
fn checked_words(run: &str) -> Vec<String> {
    let check = regex::Regex::new(r"sha256sum\s+(?:-[A-Za-z]*c[A-Za-z]*|--check)\b").unwrap();
    run_segments(run)
        .into_iter()
        .filter(|segment| check.is_match(segment))
        .flat_map(|segment| {
            segment
                .split(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .filter(|word| !word.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The value after `--name` (`--name VALUE` or `--name=VALUE`) among `args`.
fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let joined = format!("{name}=");
    args.iter().enumerate().find_map(|(index, arg)| {
        if arg == name {
            args.get(index + 1).map(String::as_str)
        } else {
            arg.strip_prefix(joined.as_str())
        }
    })
}

/// The rules for one `rustup` command (CR-01, WR-06): everything that can
/// download goes through a `file://` mirror named in that same command, an
/// install never replaces the pinned rustup, and `rustup-init` never installs
/// an unpinned default toolchain.
fn rustup_issues(command: &ParsedCommand, issues: &mut Vec<String>) {
    let args = command.args();
    let positional: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| !arg.starts_with('-') && !arg.starts_with('+'))
        .collect();
    let sub = positional.first().copied();
    let second = positional.get(1).copied();
    let read_only = matches!(
        (sub, second),
        (None | Some("help" | "which"), _)
            | (Some("toolchain" | "target" | "component"), Some("list"))
    );
    let text = &command.text;
    if !read_only
        && !command
            .env_value("RUSTUP_DIST_SERVER")
            .is_some_and(|value| value.starts_with("file://"))
    {
        issues.push(format!(
            "rustup {} without RUSTUP_DIST_SERVER=file://... in the same command, so rustup fetches its own copy of what was checked: {text}",
            sub.unwrap_or("")
        ));
    }
    let installs = matches!(sub, Some("install" | "update"))
        || (sub == Some("toolchain") && matches!(second, Some("install" | "add" | "update")));
    if installs && !args.iter().any(|arg| arg == "--no-self-update") {
        issues.push(format!(
            "rustup toolchain install without `--no-self-update` may replace the pinned rustup: {text}"
        ));
    }
}

/// `rustup-init` must not install a default toolchain: the pinned toolchains
/// come from the checked mirror, and a default would be fetched with no checksum.
fn rustup_init_issues(command: &ParsedCommand, issues: &mut Vec<String>) {
    let none = option_value(command.args(), "--default-toolchain") == Some("none");
    if !none {
        issues.push(format!(
            "rustup-init without `--default-toolchain none` installs an unpinned toolchain: {}",
            command.text
        ));
    }
}

/// The rules for one `cargo install` (or `cargo binstall`) command (CR-01, WR-06).
/// `tars` are the `(directory, archive)` pairs of every tar extraction in the
/// RUN and `checked` the words of its `sha256sum -c` lines.
fn cargo_install_issues(
    command: &ParsedCommand,
    tars: &[(Option<String>, Option<String>)],
    checked: &[String],
    issues: &mut Vec<String>,
) {
    let args = command.args();
    let text = &command.text;
    if !args.iter().any(|arg| arg == "--locked") {
        issues.push(format!("RUN cargo install lacks `--locked`: {text}"));
    }
    let exact = option_value(args, "--version").is_some_and(|value| value.starts_with('='));
    if !exact {
        issues.push(format!(
            "RUN cargo install lacks an exact `--version =`: {text}"
        ));
    }
    // The crate is installed from the .crate file that was checked in this RUN,
    // unpacked, and not fetched a second time from the registry. The directory
    // must lie under the `-C` target of a tar extraction of a file that a
    // `sha256sum -c` line in the RUN names.
    let Some(path) = option_value(args, "--path") else {
        issues.push(format!(
            "RUN cargo install lacks `--path`, so it installs from the registry instead of the verified .crate: {text}"
        ));
        return;
    };
    let bound = !path.contains("..")
        && tars.iter().any(|(directory, archive)| {
            let under = directory.as_deref().is_some_and(|dir| {
                let dir = dir.trim_end_matches('/');
                !dir.is_empty() && path.starts_with(&format!("{dir}/"))
            });
            let verified = archive
                .as_deref()
                .is_some_and(|archive| checked.iter().any(|word| word == archive));
            under && verified
        });
    if !bound {
        issues.push(format!(
            "RUN cargo install --path reads a directory that was not unpacked from a verified .crate in this RUN (needs a `sha256sum -c` of an archive, a tar extraction of that archive with `-C` above the path, and no `..`): {text}"
        ));
    }
}

/// The checks that apply to one `RUN` instruction.
fn run_issues(run: &str, issues: &mut Vec<String>) {
    let curl = regex::Regex::new(r"(?:^|[\s;&|(])curl\s+-").unwrap();
    let pipe_to_shell = regex::Regex::new(r"\|\s*(?:sudo\s+)?(?:ba|da|z)?sh\b").unwrap();
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

    // A dist server in this RUN that is not a local mirror.
    for captures in dist_server.captures_iter(run) {
        let value = captures
            .get(1)
            .map_or("", |m| m.as_str())
            .trim_matches(['"', '\'']);
        if !value.starts_with("file://") {
            issues.push(format!(
                "RUN sets RUSTUP_DIST_SERVER to `{value}`, whose scheme is not file: {run}"
            ));
        }
    }

    // WR-06: every rule below applies to each install command on its own, however
    // it is spelled, not to the RUN as a whole.
    let commands: Vec<ParsedCommand> = run_commands(run)
        .iter()
        .filter_map(|text| parse_command(text))
        .collect();
    let tars: Vec<(Option<String>, Option<String>)> =
        commands.iter().filter_map(tar_extraction).collect();
    let checked = checked_words(run);
    for command in &commands {
        match command.program() {
            "rustup" => rustup_issues(command, issues),
            "rustup-init" => rustup_init_issues(command, issues),
            "cargo-binstall" => cargo_install_issues(command, &tars, &checked, issues),
            "cargo" => {
                let sub = command
                    .args()
                    .iter()
                    .find(|arg| !arg.starts_with('-') && !arg.starts_with('+'));
                if sub.is_some_and(|sub| sub == "install" || sub == "binstall") {
                    cargo_install_issues(command, &tars, &checked, issues);
                }
            }
            _ => {}
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
        RUN curl -fsSL -o /tmp/crates/x-1.0.0.crate https://static.crates.io/crates/x/x-1.0.0.crate \\\n \
            && echo \"h  /tmp/crates/x-1.0.0.crate\" | sha256sum -c - \\\n \
            && tar -xzf /tmp/crates/x-1.0.0.crate --no-same-owner -C /tmp/crates \\\n \
            && cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x\n\
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

/// `RUN` arguments of the cargo-tools stage as the Dockerfile had them before
/// CR-01 was fixed, copied verbatim: each .crate is downloaded and checked, and
/// then `cargo install` fetches the crate again from the registry.
const CR01_CARGO_RUN: &str = r##"set -eux; \
    : "${LLVM_MAJOR:?}" "${RUST_TOOL_VERSION:?}" "${TINYCBOR_COMMIT:?}"; \
    for spec in "${C2RUST_NAME}:${C2RUST_VERSION}:${C2RUST_SHA256}" "${CARGO_MUTANTS_NAME}:${CARGO_MUTANTS_VERSION}:${CARGO_MUTANTS_SHA256}"; do \
      name="${spec%%:*}"; \
      rest="${spec#*:}"; \
      version="${rest%%:*}"; \
      sha="${rest#*:}"; \
      curl -fsSL --retry 5 --retry-delay 5 -o "/tmp/${name}-${version}.crate" "https://static.crates.io/crates/${name}/${name}-${version}.crate"; \
      echo "${sha}  /tmp/${name}-${version}.crate" | sha256sum -c -; \
    done; \
    llvm_bin="/usr/lib/llvm-${LLVM_MAJOR}/bin"; \
    export RUSTUP_TOOLCHAIN="${RUST_TOOL_VERSION}" \
           LLVM_CONFIG_PATH="${llvm_bin}/llvm-config" \
           CLANG_PATH="${llvm_bin}/clang" \
           LIBCLANG_PATH="/usr/lib/llvm-${LLVM_MAJOR}/lib"; \
    cargo install --locked --root /opt/cargo-tools "${C2RUST_NAME}" --version "=${C2RUST_VERSION}"; \
    cargo install --locked --root /opt/cargo-tools "${CARGO_MUTANTS_NAME}" --version "=${CARGO_MUTANTS_VERSION}"; \
    grep -q "${TINYCBOR_COMMIT}" "${CARGO_HOME}"/registry/src/*/c2rust-ast-exporter-"${C2RUST_VERSION}"/src/CMakeLists.txt; \
    mkdir -p /opt/cargo-tools/share/mt; \
    "${llvm_bin}/llvm-config" --version > /opt/cargo-tools/share/mt/c2rust-llvm-config-version; \
    chmod -R a+rX /opt/cargo-tools"##;

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
fn tool_03_dockerfile_installs_crates_from_verified_files() {
    let fetch = "curl -fsSL -o /tmp/crates/x-1.0.0.crate https://static.crates.io/crates/x/x-1.0.0.crate \
        && echo \"h  /tmp/crates/x-1.0.0.crate\" | sha256sum -c - \
        && tar -xzf /tmp/crates/x-1.0.0.crate --no-same-owner -C /tmp/crates";

    // What must be refused: the pre-fix RUN verbatim (a checked download that
    // `cargo install` then ignores), an install from a path nothing in the RUN put
    // there, a registry install, and a verified, unpacked install that drops
    // `--locked` or the exact version.
    let issues = dockerfile_issues(&run_fixture(CR01_CARGO_RUN));
    assert!(
        mentions(&issues, "installs from the registry"),
        "the pre-fix cargo-tools RUN must be refused, got {issues:?}"
    );

    let unverified = dockerfile_issues(&run_fixture(
        "cargo install --locked --root /r --path /src/x --version =1.0.0 x",
    ));
    assert!(
        mentions(&unverified, "not unpacked from a verified .crate"),
        "an install from an unchecked path must be refused, got {unverified:?}"
    );
    let no_check = dockerfile_issues(&run_fixture(
        "tar -xzf /tmp/crates/x-1.0.0.crate -C /tmp/crates \
         && cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
    ));
    assert!(
        mentions(&no_check, "not unpacked from a verified .crate"),
        "an unpacked but unchecked crate must be refused, got {no_check:?}"
    );
    let registry = dockerfile_issues(&run_fixture("cargo install --locked foo --version =1.0.0"));
    assert!(
        mentions(&registry, "installs from the registry"),
        "a registry install must be refused, got {registry:?}"
    );
    let not_locked = dockerfile_issues(&run_fixture(&format!(
        "{fetch} && cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x"
    )));
    assert!(
        mentions(&not_locked, "lacks `--locked`"),
        "an install without --locked must be refused, got {not_locked:?}"
    );
    let not_exact = dockerfile_issues(&run_fixture(&format!(
        "{fetch} && cargo install --locked --root /r --path /tmp/crates/x-1.0.0 x"
    )));
    assert!(
        mentions(&not_exact, "lacks an exact `--version =`"),
        "an install without an exact version must be refused, got {not_exact:?}"
    );

    // What must be accepted: download, check, unpack and install that directory.
    let accepted = dockerfile_issues(&run_fixture(&format!(
        "{fetch} && cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x"
    )));
    assert_eq!(accepted, Vec::<String>::new());

    // The real Dockerfile passes, and the check is not vacuous: some RUN installs
    // crates, and every `cargo install` in it uses `--path`.
    let dockerfile = read_repo_file("container/Dockerfile");
    assert_eq!(dockerfile_issues(&dockerfile), Vec::<String>::new());
    let installing: Vec<String> = dockerfile_instructions(&dockerfile)
        .into_iter()
        .filter(|(keyword, rest)| keyword == "RUN" && rest.contains("cargo install"))
        .map(|(_, rest)| rest)
        .collect();
    assert!(
        !installing.is_empty(),
        "no RUN runs `cargo install`; the check would be vacuous"
    );
    for run in &installing {
        for segment in run_segments(run) {
            if segment.contains("cargo install") {
                assert!(
                    segment.contains("--path"),
                    "a `cargo install` does not use --path: {segment}"
                );
            }
        }
    }
}

/// The `RUN` fixture issues, for one command line.
fn issues_of(run: &str) -> Vec<String> {
    dockerfile_issues(&run_fixture(run))
}

#[test]
fn tool_03_rustup_rules_apply_to_each_command_however_it_is_spelled() {
    let mirror = "RUSTUP_DIST_SERVER=\"file:///m\"";
    let good =
        format!("{mirror} rustup toolchain install 1.0.0 --profile minimal --no-self-update");

    // The second install is unprefixed: one prefixed install elsewhere in the RUN
    // does not excuse it, and only the unprefixed one is named.
    let second = issues_of(&format!(
        "{good}; rustup toolchain install 2.0.0 --profile minimal --no-self-update"
    ));
    assert!(
        second
            .iter()
            .any(|i| i.contains("without RUSTUP_DIST_SERVER=file://") && i.contains("2.0.0")),
        "an unprefixed second install must be refused, got {second:?}"
    );
    assert!(
        !second.iter().any(|i| i.contains("1.0.0")),
        "the prefixed install must not be named, got {second:?}"
    );

    // Every rustup verb that can download is refused without a mirror, in any
    // spelling: extra blanks, an absolute path, wrappers, a shell -c string.
    let unmirrored = [
        "rustup install 1.0.0 --no-self-update",
        "rustup toolchain add 1.0.0 --no-self-update",
        "rustup update 1.0.0 --no-self-update",
        "rustup target add thumbv7em-none-eabihf",
        "rustup component add clippy",
        "rustup default 1.0.0",
        "rustup show",
        "rustup  toolchain   install  1.0.0 --no-self-update",
        "/opt/cargo/bin/rustup toolchain install 1.0.0 --no-self-update",
        "env FOO=1 rustup toolchain install 1.0.0 --no-self-update",
        "sudo rustup toolchain install 1.0.0 --no-self-update",
        "sh -c \"rustup toolchain install 1.0.0 --no-self-update\"",
        "bash -ec 'rustup default 1.0.0'",
        "FOO=1 rustup toolchain install 1.0.0 --no-self-update",
        "RUSTUP_DIST_SERVER=https://static.rust-lang.org rustup default 1.0.0",
    ];
    for command in unmirrored {
        let issues = issues_of(command);
        assert!(
            mentions(&issues, "without RUSTUP_DIST_SERVER=file://"),
            "`{command}` must be refused, got {issues:?}"
        );
    }

    // `install`, `update` and `toolchain add|update` also need --no-self-update.
    for command in [
        format!("{mirror} rustup install 1.0.0"),
        format!("{mirror} rustup update 1.0.0"),
        format!("{mirror} rustup toolchain add 1.0.0"),
        format!("{mirror} rustup toolchain update 1.0.0"),
    ] {
        assert!(
            mentions(&issues_of(&command), "--no-self-update"),
            "`{command}` must be refused"
        );
    }

    // Mirrored commands, and read-only ones, are accepted.
    for command in [
        good.clone(),
        format!("{mirror} rustup default 1.0.0"),
        format!("{mirror} rustup target add thumbv7em-none-eabihf"),
        "rustup toolchain list".to_owned(),
        "rustup --version".to_owned(),
    ] {
        assert_eq!(
            issues_of(&command),
            Vec::<String>::new(),
            "`{command}` must be accepted"
        );
    }
}

#[test]
fn tool_03_rustup_init_must_not_install_a_default_toolchain() {
    let check = "curl -fsSL -o /tmp/rustup-init https://example.org/rustup-init \
        && echo \"h  /tmp/rustup-init\" | sha256sum -c - && chmod +x /tmp/rustup-init";
    for init in [
        "/tmp/rustup-init -y --profile minimal",
        "/tmp/rustup-init -y --default-toolchain stable",
        "/tmp/rustup-init -y --default-toolchain",
        "sh /tmp/rustup-init -y",
        "rustup-init -y",
    ] {
        let issues = issues_of(&format!("{check} && {init}"));
        assert!(
            mentions(&issues, "--default-toolchain none"),
            "`{init}` must be refused, got {issues:?}"
        );
    }
    for init in [
        "/tmp/rustup-init -y --default-toolchain none --profile minimal",
        "/tmp/rustup-init -y --default-toolchain=none",
    ] {
        assert_eq!(
            issues_of(&format!("{check} && {init}")),
            Vec::<String>::new(),
            "`{init}` must be accepted"
        );
    }
    // Handling the file is not running it.
    assert_eq!(
        issues_of(&format!("{check} && rm -f /tmp/rustup-init")),
        Vec::<String>::new()
    );
}

#[test]
fn tool_03_cargo_install_rules_apply_to_each_command_however_it_is_spelled() {
    let fetch = "curl -fsSL -o /tmp/crates/x-1.0.0.crate https://static.crates.io/crates/x/x-1.0.0.crate \
        && echo \"h  /tmp/crates/x-1.0.0.crate\" | sha256sum -c - \
        && tar -xzf /tmp/crates/x-1.0.0.crate --no-same-owner -C /tmp/crates";
    let good = "cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x";
    assert_eq!(
        issues_of(&format!("{fetch} && {good}")),
        Vec::<String>::new()
    );

    // Spellings that used to slip past the literal `cargo install`.
    for command in [
        "cargo  install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        "cargo +1.99.0 install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        "/opt/cargo/bin/cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        "env CARGO_HOME=/c cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        "sh -c \"cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x\"",
    ] {
        let issues = issues_of(&format!("{fetch} && {command}"));
        assert!(
            mentions(&issues, "lacks `--locked`"),
            "`{command}` must be refused, got {issues:?}"
        );
    }
    for command in [
        "cargo binstall --locked --version =1.0.0 x",
        "cargo +1.99.0 install --locked --version =1.0.0 x",
        "cargo-binstall --locked --version =1.0.0 x",
        "cargo  install  --locked x --version =1.0.0",
    ] {
        let issues = issues_of(&format!("{fetch} && {command}"));
        assert!(
            mentions(&issues, "installs from the registry"),
            "`{command}` must be refused, got {issues:?}"
        );
    }

    // One of two installs lacks --locked: only that one is named.
    let two = issues_of(&format!(
        "{fetch} && {good}; cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 y"
    ));
    assert!(
        two.iter()
            .any(|i| i.contains("lacks `--locked`") && i.ends_with(" y")),
        "the unlocked install must be named, got {two:?}"
    );
    assert!(
        !two.iter()
            .any(|i| i.contains("lacks `--locked`") && i.ends_with(" x")),
        "the locked install must not be named, got {two:?}"
    );

    // --path must be tied to the checked archive, not to any directory.
    let elsewhere = issues_of(&format!(
        "{fetch} && cargo install --locked --root /r --path /elsewhere/x-1.0.0 --version =1.0.0 x"
    ));
    assert!(
        mentions(&elsewhere, "not unpacked from a verified .crate"),
        "a path outside the tar -C directory must be refused, got {elsewhere:?}"
    );
    let traversal = issues_of(&format!(
        "{fetch} && cargo install --locked --root /r --path /tmp/crates/../elsewhere --version =1.0.0 x"
    ));
    assert!(
        mentions(&traversal, "not unpacked from a verified .crate"),
        "a path with .. must be refused, got {traversal:?}"
    );
    // An unrelated file is checked and an unrelated archive is unpacked.
    let unrelated = issues_of(
        "echo \"h  /tmp/a.crate\" | sha256sum -c - \
         && tar -xzf /tmp/b.crate -C /tmp/crates \
         && cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
    );
    assert!(
        mentions(&unrelated, "not unpacked from a verified .crate"),
        "an unpacked archive that was never checked must be refused, got {unrelated:?}"
    );
    // The checked archive is unpacked somewhere else than the install path.
    let other_dir = issues_of(
        "echo \"h  /tmp/b.crate\" | sha256sum -c - \
         && tar -xzf /tmp/b.crate -C /tmp/other \
         && cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
    );
    assert!(
        mentions(&other_dir, "not unpacked from a verified .crate"),
        "a different extraction directory must be refused, got {other_dir:?}"
    );
    // Long and clustered tar spellings are understood.
    for tar in [
        "tar --extract --file=/tmp/b.crate --directory=/tmp/crates",
        "tar --extract --file /tmp/b.crate --directory /tmp/crates",
        "tar -x -f /tmp/b.crate -C /tmp/crates",
        "tar xzf /tmp/b.crate -C /tmp/crates",
    ] {
        let issues = issues_of(&format!(
            "echo \"h  /tmp/b.crate\" | sha256sum -c - && {tar} \
             && cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x"
        ));
        assert_eq!(issues, Vec::<String>::new(), "`{tar}` must be accepted");
    }
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
    let guard = "if: ${{ github.event_name == 'push' && (github.ref == 'refs/heads/main' || github.ref == 'refs/heads/claude/gsd-discuss-phase-1-mzdw5v') }}";
    assert_eq!(
        workflow.matches(guard).count(),
        2,
        "exactly the login and push steps carry the publish guard"
    );
    assert!(
        !workflow.contains("github.event_name != 'pull_request'"),
        "publishing must not be gated on `!= pull_request` (any branch, schedule, dispatch)"
    );
    assert!(
        image_a[..login]
            .rfind(guard)
            .is_some_and(|at| login - at < 200),
        "the login step must run only for pushes to main or the phase branch"
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
        "the push step must itself run only for pushes to main or the phase branch"
    );
    assert!(image_a.contains("ghcr.io/sat-wik/migrationtool-toolchain"));
    assert!(image_a.contains("{{index .RepoDigests 0}}"));
    assert!(
        !image_a[push..].contains("docker build") && !image_a[push..].contains("buildx build"),
        "the pushed image is never rebuilt for publishing"
    );

    // A newer run may cancel only a pull_request run, never a push that is
    // between `docker push` and the compare job (WR-04).
    assert!(
        workflow.contains("cancel-in-progress: ${{ github.event_name == 'pull_request' }}"),
        "cancel-in-progress must be limited to pull_request runs"
    );
    assert!(!workflow.contains("cancel-in-progress: true"));

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

    // The record also holds the two crates, matched by the same whole-line rule.
    assert!(
        image_a.contains("/tmp/crates/"),
        "image-a must match the /tmp/crates/ lines of the build record"
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
