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

/// The first two positional words after `rustup` (`toolchain install`, `default`).
fn rustup_words(command: &ParsedCommand) -> (Option<&str>, Option<&str>) {
    let mut positional = command
        .args()
        .iter()
        .map(String::as_str)
        .filter(|arg| !arg.starts_with('-') && !arg.starts_with('+'));
    let sub = positional.next();
    (sub, positional.next())
}

/// True for the `rustup` commands that only read local state.
fn rustup_read_only(command: &ParsedCommand) -> bool {
    matches!(
        rustup_words(command),
        (None | Some("help" | "which"), _)
            | (Some("toolchain" | "target" | "component"), Some("list"))
    )
}

/// The rules for one `rustup` command (CR-01, WR-06): everything that can
/// download goes through a `file://` mirror named in that same command, an
/// install never replaces the pinned rustup, and `rustup-init` never installs
/// an unpinned default toolchain.
fn rustup_issues(command: &ParsedCommand, issues: &mut Vec<String>) {
    let args = command.args();
    let (sub, second) = rustup_words(command);
    let read_only = rustup_read_only(command);
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
    // The crate is installed from the .crate file that was checked, unpacked, and
    // not fetched a second time from the registry. The directory must lie under
    // the `-C` target of a tar extraction of a file that a `sha256sum -c` line
    // names. For an install in a `RUN --network=none` the checked download may sit
    // in an earlier RUN of the same stage.
    let Some(path) = option_value(args, "--path") else {
        issues.push(format!(
            "RUN cargo install lacks `--path`, so it installs from the registry instead of the verified .crate: {text}"
        ));
        return;
    };
    if !path_bound(path, tars, checked) {
        issues.push(format!(
            "RUN cargo install --path reads a directory that was not unpacked from a verified .crate (needs a `sha256sum -c` of an archive, a tar extraction of that archive with `-C` above the path, and no `..`): {text}"
        ));
    }
}

/// True when `path` lies under the `-C` directory of a tar extraction of an
/// archive that a `sha256sum -c` line names, and contains no `..`.
fn path_bound(path: &str, tars: &[(Option<String>, Option<String>)], checked: &[String]) -> bool {
    !path.contains("..")
        && tars.iter().any(|(directory, archive)| {
            let under = directory.as_deref().is_some_and(|dir| {
                let dir = dir.trim_end_matches('/');
                !dir.is_empty() && path.starts_with(&format!("{dir}/"))
            });
            let verified = archive
                .as_deref()
                .is_some_and(|archive| checked.iter().any(|word| word == archive));
            under && verified
        })
}

/// The rules for one `cargo fetch` (WR-06): it reads the dependency lock of a
/// verified, unpacked `.crate` and nothing else, so that the offline install
/// that follows has exactly the registry files that lock names.
fn cargo_fetch_issues(
    command: &ParsedCommand,
    tars: &[(Option<String>, Option<String>)],
    checked: &[String],
    issues: &mut Vec<String>,
) {
    let args = command.args();
    let text = &command.text;
    if !args.iter().any(|arg| arg == "--locked") {
        issues.push(format!("RUN cargo fetch lacks `--locked`: {text}"));
    }
    match option_value(args, "--manifest-path") {
        None => issues.push(format!(
            "RUN cargo fetch lacks `--manifest-path`, so it resolves an unverified manifest: {text}"
        )),
        Some(path) if !path_bound(path, tars, checked) => issues.push(format!(
            "RUN cargo fetch --manifest-path reads a manifest that was not unpacked from a verified .crate: {text}"
        )),
        Some(_) => {}
    }
}

/// `(name, reason)` of install commands that still run with network access.
/// Empty unless a step proved unable to run offline; each entry needs a
/// `# Remains networked:` comment in the Dockerfile that carries its reason.
const NETWORKED_INSTALLS: &[(&str, &str)] = &[];

/// `(name, reason)` of other build steps (not installs) that still run with
/// network access, under the same rule as [`NETWORKED_INSTALLS`]. Empty since
/// WR-08 pinned the klee-uclibc locale tarball: `tool_03_dockerfile_pins_the_klee_uclibc_locale_tarball`
/// requires it to stay empty.
const NETWORKED_STEPS: &[(&str, &str)] = &[];

/// Evidence earlier `RUN`s of one stage gathered, for a later offline install.
#[derive(Default)]
struct StageEvidence {
    /// `(directory, archive)` of every tar extraction.
    tars: Vec<(Option<String>, Option<String>)>,
    /// Words of every `sha256sum -c` line.
    checked: Vec<String>,
}

/// Leading `--flag` tokens of a `RUN` instruction, and the command text after them.
fn run_flags(rest: &str) -> (Vec<&str>, &str) {
    let mut flags = Vec::new();
    let mut body = rest.trim_start();
    while body.starts_with("--") {
        let end = body.find(char::is_whitespace).unwrap_or(body.len());
        flags.push(&body[..end]);
        body = body[end..].trim_start();
    }
    (flags, body)
}

/// True for a command that installs a toolchain, target, component or crate.
fn is_install(command: &ParsedCommand) -> bool {
    match command.program() {
        "rustup-init" | "cargo-binstall" => true,
        "rustup" => !rustup_read_only(command),
        "cargo" => command
            .args()
            .iter()
            .find(|arg| !arg.starts_with('-') && !arg.starts_with('+'))
            .is_some_and(|sub| sub == "install" || sub == "binstall"),
        _ => false,
    }
}

/// The first positional word after `git`'s own options (`clone`, `fetch`, ...).
fn git_subcommand(command: &ParsedCommand) -> Option<&str> {
    let mut args = command.args().iter();
    while let Some(arg) = args.next() {
        if arg == "-C" || arg == "-c" {
            args.next();
        } else if !arg.starts_with('-') {
            return Some(arg.as_str());
        }
    }
    None
}

/// True for a command that reaches the network to download something.
fn is_download(command: &ParsedCommand) -> bool {
    match command.program() {
        "curl" | "wget" | "apt-get" | "apt" | "aptitude" => true,
        "git" => matches!(
            git_subcommand(command),
            Some("clone" | "fetch" | "pull" | "ls-remote" | "submodule")
        ),
        "cargo" => command
            .args()
            .iter()
            .find(|arg| !arg.starts_with('-') && !arg.starts_with('+'))
            .is_some_and(|sub| sub == "fetch" || sub == "update"),
        _ => false,
    }
}

/// The checks that apply to one `RUN` instruction (`rest` is everything after
/// the keyword, flags included). `evidence` is what earlier RUNs of the same
/// stage checked and unpacked; it counts only for an install in a
/// `RUN --network=none`, and this RUN's own evidence is added to it afterwards.
fn run_issues(rest: &str, evidence: &mut StageEvidence, issues: &mut Vec<String>) {
    let (flags, run) = run_flags(rest);
    let mut offline = false;
    for flag in flags {
        match flag.strip_prefix("--network=") {
            Some("none") => offline = true,
            Some(other) => issues.push(format!(
                "RUN --network={other} keeps network access: only --network=none is accepted: {run}"
            )),
            None => {}
        }
    }
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
    let own_tars: Vec<(Option<String>, Option<String>)> =
        commands.iter().filter_map(tar_extraction).collect();
    let own_checked = checked_words(run);
    let (tars, checked) = if offline {
        (
            [evidence.tars.as_slice(), own_tars.as_slice()].concat(),
            [evidence.checked.as_slice(), own_checked.as_slice()].concat(),
        )
    } else {
        (own_tars.clone(), own_checked.clone())
    };
    for command in &commands {
        if is_install(command) {
            let excused = NETWORKED_INSTALLS
                .iter()
                .any(|(marker, reason)| run.contains(marker) && !reason.is_empty());
            if !offline && !excused {
                issues.push(format!(
                    "RUN installs in a networked RUN; the install must run in a RUN --network=none after a networked RUN that downloads and checks its inputs: {}",
                    command.text
                ));
            }
        }
        if offline && is_download(command) {
            issues.push(format!(
                "RUN --network=none runs a download command, which can only fail or hide a fetch: {}",
                command.text
            ));
        }
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
                } else if sub.is_some_and(|sub| sub == "fetch") {
                    cargo_fetch_issues(command, &tars, &checked, issues);
                }
            }
            _ => {}
        }
    }
    evidence.tars.extend(own_tars);
    evidence.checked.extend(own_checked);
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
    let mut evidence = StageEvidence::default();
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
                evidence = StageEvidence::default();
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
            "RUN" => run_issues(&rest, &mut evidence, &mut issues),
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
            && tar -xzf /tmp/crates/x-1.0.0.crate --no-same-owner -C /tmp/crates\n\
        RUN --network=none cargo install --locked --offline --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x\n\
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

/// A fixture Dockerfile holding one `RUN --network=none` of `run`.
fn offline_fixture(run: &str) -> String {
    format!("{FIXTURE_FROM}RUN --network=none {run}\n")
}

/// A fixture stage with a networked `RUN` (the download) and then a
/// `RUN --network=none` (the install) of the same stage, the shape WR-06 asks for.
fn split_fixture(download: &str, offline: &str) -> String {
    format!("{FIXTURE_FROM}RUN {download}\nRUN --network=none {offline}\n")
}

/// Issues of the split fixture.
fn split_issues(download: &str, offline: &str) -> Vec<String> {
    dockerfile_issues(&split_fixture(download, offline))
}

/// Issues of a fixture whose only RUN is a `RUN --network=none` of `run`.
fn offline_issues_of(run: &str) -> Vec<String> {
    dockerfile_issues(&offline_fixture(run))
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
    let only_first = split_issues(
        check,
        "RUSTUP_DIST_SERVER=\"file:///m\" rustup toolchain install 1.0.0 --profile minimal --no-self-update; \
         RUSTUP_DIST_SERVER=\"file:///m\" rustup toolchain install 2.0.0 --profile minimal",
    );
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
    let accepted = split_issues(
        &format!("mkdir -p /m/dist && {check}"),
        "RUSTUP_DIST_SERVER=\"file:///m\" rustup toolchain install 1.0.0 --profile minimal --no-self-update",
    );
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
    let accepted = split_issues(
        fetch,
        "cargo install --locked --offline --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
    );
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
    let second = offline_issues_of(&format!(
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
        let issues = offline_issues_of(command);
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
            mentions(&offline_issues_of(&command), "--no-self-update"),
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
            offline_issues_of(&command),
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
        let issues = split_issues(check, init);
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
            split_issues(check, init),
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
    assert_eq!(split_issues(fetch, good), Vec::<String>::new());

    // Spellings that used to slip past the literal `cargo install`.
    for command in [
        "cargo  install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        "cargo +1.99.0 install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        "/opt/cargo/bin/cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        "env CARGO_HOME=/c cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        "sh -c \"cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x\"",
    ] {
        let issues = split_issues(fetch, command);
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
        let issues = split_issues(fetch, command);
        assert!(
            mentions(&issues, "installs from the registry"),
            "`{command}` must be refused, got {issues:?}"
        );
    }

    // One of two installs lacks --locked: only that one is named.
    let two = split_issues(
        fetch,
        &format!("{good}; cargo install --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 y"),
    );
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
    let elsewhere = split_issues(
        fetch,
        "cargo install --locked --root /r --path /elsewhere/x-1.0.0 --version =1.0.0 x",
    );
    assert!(
        mentions(&elsewhere, "not unpacked from a verified .crate"),
        "a path outside the tar -C directory must be refused, got {elsewhere:?}"
    );
    let traversal = split_issues(
        fetch,
        "cargo install --locked --root /r --path /tmp/crates/../elsewhere --version =1.0.0 x",
    );
    assert!(
        mentions(&traversal, "not unpacked from a verified .crate"),
        "a path with .. must be refused, got {traversal:?}"
    );
    // An unrelated file is checked and an unrelated archive is unpacked.
    let unrelated = split_issues(
        "echo \"h  /tmp/a.crate\" | sha256sum -c - \
         && tar -xzf /tmp/b.crate -C /tmp/crates",
        "cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
    );
    assert!(
        mentions(&unrelated, "not unpacked from a verified .crate"),
        "an unpacked archive that was never checked must be refused, got {unrelated:?}"
    );
    // The checked archive is unpacked somewhere else than the install path.
    let other_dir = split_issues(
        "echo \"h  /tmp/b.crate\" | sha256sum -c - \
         && tar -xzf /tmp/b.crate -C /tmp/other",
        "cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
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
        let issues = split_issues(
            &format!("echo \"h  /tmp/b.crate\" | sha256sum -c - && {tar}"),
            "cargo install --locked --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x",
        );
        assert_eq!(issues, Vec::<String>::new(), "`{tar}` must be accepted");
    }
}

/// The `RUN` instructions of stage `stage` as `(flags, body)`, in order.
fn stage_runs(text: &str, stage: &str) -> Vec<(Vec<String>, String)> {
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
            "RUN" if current == stage => {
                let (flags, body) = run_flags(&rest);
                found.push((
                    flags.iter().map(|flag| (*flag).to_owned()).collect(),
                    body.to_owned(),
                ));
            }
            _ => {}
        }
    }
    found
}

/// True when the flags of a `RUN` include `--network=none`.
fn is_offline(flags: &[String]) -> bool {
    flags.iter().any(|flag| flag == "--network=none")
}

#[test]
fn tool_03_dockerfile_installs_only_in_network_none_runs() {
    let fetch = "curl -fsSL -o /tmp/crates/x-1.0.0.crate https://static.crates.io/crates/x/x-1.0.0.crate \
        && echo \"h  /tmp/crates/x-1.0.0.crate\" | sha256sum -c - \
        && tar -xzf /tmp/crates/x-1.0.0.crate --no-same-owner -C /tmp/crates";
    let mirror = "RUSTUP_DIST_SERVER=\"file:///m\"";
    let install =
        "cargo install --locked --offline --root /r --path /tmp/crates/x-1.0.0 --version =1.0.0 x";

    // An install in a plain RUN is refused, whatever else the RUN checked, and the
    // message names the command, not the whole RUN.
    let networked = [
        format!("{mirror} rustup toolchain install 1.0.0 --profile minimal --no-self-update"),
        format!("{mirror} rustup default 1.0.0"),
        format!("{mirror} rustup target add thumbv7em-none-eabihf"),
        format!("{mirror} rustup component add clippy"),
        "/tmp/rustup-init -y --default-toolchain none".to_owned(),
        "rustup-init -y --default-toolchain none".to_owned(),
        install.to_owned(),
        "cargo binstall --locked --version =1.0.0 x".to_owned(),
    ];
    for command in &networked {
        let issues = issues_of(&format!("echo prelude-marker; {fetch}; {command}"));
        assert!(
            issues
                .iter()
                .any(|i| i.contains("networked RUN") && i.contains(command.as_str())),
            "`{command}` in a networked RUN must be refused by name, got {issues:?}"
        );
        assert!(
            !issues.iter().any(|i| i.contains("prelude-marker")),
            "the message must name the command, not the RUN: {issues:?}"
        );
    }

    // Other network modes keep the network.
    for mode in ["host", "default"] {
        let issues = dockerfile_issues(&format!(
            "{FIXTURE_FROM}RUN --network={mode} echo harmless\n"
        ));
        assert!(
            mentions(&issues, &format!("--network={mode}")),
            "RUN --network={mode} must be refused, got {issues:?}"
        );
    }

    // A download inside RUN --network=none can only fail or hide a fetch.
    for download in [
        "curl -fsSL -o /x https://example.org/x",
        "git clone https://example.org/r /r",
        "git -C /r fetch origin",
        "apt-get install --yes make",
    ] {
        let issues = offline_issues_of(download);
        assert!(
            issues
                .iter()
                .any(|i| i.contains("RUN --network=none") && i.contains(download)),
            "`{download}` in an offline RUN must be refused, got {issues:?}"
        );
    }

    // The verified tar evidence has to be in the same stage as the offline install.
    let other_stage =
        format!("{FIXTURE_FROM}RUN {fetch}\nFROM a AS two\nRUN --network=none {install}\n");
    assert!(
        mentions(
            &dockerfile_issues(&other_stage),
            "not unpacked from a verified .crate"
        ),
        "evidence from an earlier stage must not bind an install"
    );
    // A networked install does not borrow evidence from an earlier RUN either.
    let split_but_networked = format!("{FIXTURE_FROM}RUN {fetch}\nRUN {install}\n");
    let issues = dockerfile_issues(&split_but_networked);
    assert!(
        mentions(&issues, "networked RUN") && mentions(&issues, "not unpacked from a verified"),
        "a networked install with evidence in an earlier RUN must be refused, got {issues:?}"
    );

    // What must be accepted.
    let accepted: [(&str, String, String); 3] = [
        ("cargo", fetch.to_owned(), install.to_owned()),
        (
            "rustup mirror",
            "mkdir -p /m/dist && curl -fsSL -o /m/dist/c.toml https://static.rust-lang.org/dist/c.toml \
             && echo \"h  /m/dist/c.toml\" | sha256sum -c -"
                .to_owned(),
            format!("{mirror} rustup toolchain install 1.0.0 --profile minimal --no-self-update"),
        ),
        (
            "rustup-init",
            "curl -fsSL -o /tmp/rustup-init https://example.org/rustup-init \
             && echo \"h  /tmp/rustup-init\" | sha256sum -c -"
                .to_owned(),
            "/tmp/rustup-init -y --default-toolchain none".to_owned(),
        ),
    ];
    for (name, download, offline) in &accepted {
        assert_eq!(
            split_issues(download, offline),
            Vec::<String>::new(),
            "the split {name} shape must be accepted"
        );
    }
    for readonly in [
        "rustup toolchain list",
        "rustup --version",
        "rustup which rustc",
    ] {
        assert_eq!(issues_of(readonly), Vec::<String>::new(), "{readonly}");
        assert_eq!(
            offline_issues_of(readonly),
            Vec::<String>::new(),
            "{readonly}"
        );
    }

    // The real Dockerfile.
    let dockerfile = read_repo_file("container/Dockerfile");
    assert_eq!(dockerfile_issues(&dockerfile), Vec::<String>::new());
    let all_runs: Vec<(Vec<String>, String)> = dockerfile_instructions(&dockerfile)
        .into_iter()
        .filter(|(keyword, _)| keyword == "RUN")
        .map(|(_, rest)| {
            let (flags, body) = run_flags(&rest);
            (
                flags.iter().map(|flag| (*flag).to_owned()).collect(),
                body.to_owned(),
            )
        })
        .collect();
    let offline_runs = all_runs
        .iter()
        .filter(|(flags, _)| is_offline(flags))
        .count();
    assert!(
        offline_runs >= 6,
        "expected at least 6 RUN --network=none instructions, found {offline_runs}"
    );
    let comments: Vec<&str> = dockerfile
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("# Remains networked:"))
        .collect();
    for marker in [
        "rustup-init -y",
        "rustup toolchain install",
        "cargo install",
        "tar -xJf /tmp/arm-gnu-toolchain.tar.xz",
        "--make-llvm-lib",
        "cd /src/klee-uclibc && make",
        "cmake --build /build/klee",
    ] {
        let mut seen = false;
        for (flags, body) in &all_runs {
            if !body.contains(marker) {
                continue;
            }
            seen = true;
            let excused = NETWORKED_INSTALLS
                .iter()
                .chain(NETWORKED_STEPS)
                .any(|(name, reason)| {
                    marker.contains(name)
                        && !reason.is_empty()
                        && comments.iter().any(|comment| comment.contains(reason))
                });
            assert!(
                is_offline(flags) || excused,
                "the RUN with `{marker}` must be RUN --network=none, or be listed with a reason that a `# Remains networked:` comment repeats"
            );
        }
        assert!(
            seen,
            "no RUN contains `{marker}`; the check would be vacuous"
        );
    }
    // Every listed exception carries a non-empty reason and a comment.
    for (name, reason) in NETWORKED_INSTALLS.iter().chain(NETWORKED_STEPS) {
        assert!(!reason.is_empty(), "{name} is excused without a reason");
        assert!(
            comments.iter().any(|comment| comment.contains(reason)),
            "{name}: no `# Remains networked:` comment repeats the reason"
        );
    }
}

/// WR-08: the one download the klee-uclibc `make` used to do on its own is a
/// pinned archive, fetched and checked in a networked RUN, and `make` runs in a
/// `RUN --network=none`. Nothing is excused any more.
#[test]
fn tool_03_dockerfile_pins_the_klee_uclibc_locale_tarball() {
    // The pin: an https archive whose file name carries its version, with a
    // well-formed sha256.
    let pins = container_pins();
    let Some(SourcePin::Archive {
        version,
        url,
        sha256,
    }) = pins.source.get("klee_uclibc_locale")
    else {
        panic!("pins.toml needs [source.klee_uclibc_locale] with kind = \"archive\"");
    };
    assert!(url.starts_with("https://"), "{url}");
    assert_eq!(
        url.rsplit('/').next(),
        Some(format!("uClibc-locale-{version}.tgz").as_str()),
        "the URL must end in the file name the klee-uclibc Makefile looks for"
    );
    assert!(
        sha256.len() == 64
            && sha256
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        "{sha256}"
    );

    // No step keeps network access for a reason, and none says it does.
    assert!(NETWORKED_INSTALLS.is_empty(), "{NETWORKED_INSTALLS:?}");
    assert!(NETWORKED_STEPS.is_empty(), "{NETWORKED_STEPS:?}");
    let dockerfile = read_repo_file("container/Dockerfile");
    assert!(
        !dockerfile
            .lines()
            .any(|line| line.trim_start().starts_with("# Remains networked:")),
        "a `# Remains networked:` label is back in the Dockerfile"
    );

    // The klee-build stage: the clone RUN downloads and checks the tarball into
    // the directory the Makefile reads, and the make RUN is offline and later.
    let runs = stage_runs(&dockerfile, "klee-build");
    let position = |needle: &str| runs.iter().position(|(_, body)| body.contains(needle));
    let download =
        position("${KLEE_UCLIBC_LOCALE_URL}").expect("no RUN downloads the locale tarball");
    let (download_flags, download_body) = &runs[download];
    assert!(
        !is_offline(download_flags),
        "the download RUN cannot be offline"
    );
    assert!(
        download_body.contains("curl")
            && download_body.contains("/src/klee-uclibc/extra/locale/")
            && download_body.contains("${KLEE_UCLIBC_LOCALE_SHA256}")
            && download_body.contains("sha256sum -c"),
        "the locale tarball must be checked against its pin in the RUN that downloads it: {download_body}"
    );
    let make = position("cd /src/klee-uclibc && make").expect("no RUN runs the klee-uclibc make");
    assert!(
        is_offline(&runs[make].0),
        "the klee-uclibc make must be RUN --network=none"
    );
    assert!(
        download < make,
        "the tarball must be in place before make runs"
    );
}

#[test]
fn tool_03_dockerfile_fetches_crate_dependencies_before_the_offline_install() {
    let fetch = "curl -fsSL -o /tmp/crates/x-1.0.0.crate https://static.crates.io/crates/x/x-1.0.0.crate \
        && echo \"h  /tmp/crates/x-1.0.0.crate\" | sha256sum -c - \
        && tar -xzf /tmp/crates/x-1.0.0.crate --no-same-owner -C /tmp/crates";

    // cargo fetch must be locked and must read a verified, unpacked manifest.
    let unlocked = issues_of(&format!(
        "{fetch} && cargo fetch --manifest-path /tmp/crates/x-1.0.0/Cargo.toml"
    ));
    assert!(
        mentions(&unlocked, "cargo fetch lacks `--locked`"),
        "{unlocked:?}"
    );
    let elsewhere = issues_of(&format!(
        "{fetch} && cargo fetch --locked --manifest-path /src/x/Cargo.toml"
    ));
    assert!(
        mentions(&elsewhere, "not unpacked from a verified .crate"),
        "{elsewhere:?}"
    );
    let traversal = issues_of(&format!(
        "{fetch} && cargo fetch --locked --manifest-path /tmp/crates/../x/Cargo.toml"
    ));
    assert!(
        mentions(&traversal, "not unpacked from a verified .crate"),
        "{traversal:?}"
    );
    let no_path = issues_of(&format!("{fetch} && cargo fetch --locked"));
    assert!(mentions(&no_path, "lacks `--manifest-path`"), "{no_path:?}");
    let good = issues_of(&format!(
        "{fetch} && cargo fetch --locked --manifest-path /tmp/crates/x-1.0.0/Cargo.toml"
    ));
    assert_eq!(good, Vec::<String>::new());
    // A fetch is a download: it cannot sit in a RUN --network=none.
    let offline_fetch =
        offline_issues_of("cargo fetch --locked --manifest-path /tmp/crates/x/Cargo.toml");
    assert!(mentions(
        &offline_fetch,
        "RUN --network=none runs a download"
    ));

    // The real cargo-tools stage: the networked RUN fetches the dependencies of
    // each verified crate and pre-clones tinycbor at the pin; the offline RUN
    // installs with --offline and redirects the tinycbor clone to the local copy.
    let dockerfile = read_repo_file("container/Dockerfile");
    let runs = stage_runs(&dockerfile, "cargo-tools");
    let networked: Vec<&String> = runs
        .iter()
        .filter(|(flags, _)| !is_offline(flags))
        .map(|(_, body)| body)
        .collect();
    let offline: Vec<&String> = runs
        .iter()
        .filter(|(flags, _)| is_offline(flags))
        .map(|(_, body)| body)
        .collect();
    assert_eq!(networked.len(), 1, "one networked RUN downloads the inputs");
    assert_eq!(offline.len(), 1, "one RUN --network=none builds them");
    for (name, version) in [
        ("C2RUST_NAME", "C2RUST_VERSION"),
        ("CARGO_MUTANTS_NAME", "CARGO_MUTANTS_VERSION"),
    ] {
        let fetch = format!(
            "cargo fetch --locked --manifest-path \"/tmp/crates/${{{name}}}-${{{version}}}/Cargo.toml\""
        );
        assert!(
            networked[0].contains(&fetch),
            "the networked RUN lacks `{fetch}`"
        );
        let install = format!(
            "cargo install --locked --offline --root /opt/cargo-tools --path \"/tmp/crates/${{{name}}}-${{{version}}}\""
        );
        assert!(
            offline[0].contains(&install),
            "the offline RUN lacks `{install}`"
        );
    }
    assert!(
        !offline[0].contains("curl") && !networked[0].contains("cargo install"),
        "downloads stay in the networked RUN and installs in the offline one"
    );
    // tinycbor: cloned and checked in the networked RUN, redirected offline.
    for needle in [
        "git clone --no-checkout https://github.com/intel/tinycbor.git /tmp/tinycbor",
        "git -C /tmp/tinycbor checkout --detach \"${TINYCBOR_COMMIT}\"",
        "git -C /tmp/tinycbor rev-parse HEAD",
        "git -C /tmp/tinycbor branch mt-pinned \"${TINYCBOR_COMMIT}\"",
    ] {
        assert!(
            networked[0].contains(needle),
            "networked RUN lacks `{needle}`"
        );
    }
    for needle in [
        "GIT_CONFIG_COUNT=1",
        "GIT_CONFIG_KEY_0=\"url.file:///tmp/tinycbor.insteadOf\"",
        "GIT_CONFIG_VALUE_0=\"https://github.com/intel/tinycbor.git\"",
        "grep -qF \"https://github.com/intel/tinycbor.git\"",
        "grep -q \"${TINYCBOR_COMMIT}\"",
    ] {
        assert!(offline[0].contains(needle), "offline RUN lacks `{needle}`");
    }
}

#[test]
fn tool_03_dockerfile_checks_each_recorded_archive_against_its_manifest() {
    // WR-07: before the first install, the build proves that the mirror holds
    // exactly the archives recorded, each one a pair its pin-verified manifest names.
    let dockerfile = read_repo_file("container/Dockerfile");
    let runs = stage_runs(&dockerfile, "rustup");
    let (flags, install) = runs
        .iter()
        .find(|(_, body)| body.contains("rustup toolchain install"))
        .expect("a RUN installs the toolchains");
    assert!(
        is_offline(flags),
        "the RUN with `rustup toolchain install` must be RUN --network=none"
    );
    let first = install.find("rustup toolchain install").unwrap();
    let before = &install[..first];
    for needle in [
        "sha256sum -c",
        "${RUST_TOOL_CHANNEL_SHA256}",
        "${RUST_BITCODE_CHANNEL_SHA256}",
        "pin-binding.sha256",
        "xz_hash",
        "xz_url",
        "exit 1",
    ] {
        assert!(
            before.contains(needle),
            "the install RUN must contain `{needle}` before its first install"
        );
    }
    // The pin-verified manifests are the ones kept in the image for the CI gate.
    let (_, mirror) = runs
        .iter()
        .find(|(_, body)| body.contains("mirror() {"))
        .expect("a RUN builds the mirror");
    assert!(
        !mirror.contains("rustup toolchain install") && !mirror.contains("rustup default"),
        "the mirror RUN only downloads"
    );
    let kept = mirror
        .find("/opt/cargo-tools/share/mt/channel-rust-")
        .expect("the mirror RUN keeps a copy of each verified manifest");
    let checked = mirror
        .find("sha256sum -c -")
        .expect("the mirror RUN checks the manifest");
    assert!(
        checked < kept,
        "the manifest copy must be taken after its sha256sum -c"
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

/// Text positions of `needles` in `haystack`, in the order given; panics naming a
/// missing needle. Used to assert the order of the steps of a job.
fn positions(haystack: &str, what: &str, needles: &[&str]) -> Vec<usize> {
    needles
        .iter()
        .map(|needle| {
            haystack
                .find(needle)
                .unwrap_or_else(|| panic!("{what} lacks `{needle}`"))
        })
        .collect()
}

/// The publish guard every publishing step and job repeats: a push to `main`
/// only, never a pull_request, schedule, dispatch or any other branch (WR-04,
/// WR-09). No other ref may ever be named in it.
const PUBLISH_GUARD: &str =
    "if: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}";

#[test]
fn tool_03_workflow_publishes_by_digest_with_least_privilege() {
    let workflow = read_repo_file(".github/workflows/container.yml");
    let image_a = workflow_job_text(&workflow, "image-a");
    let publish = workflow_job_text(&workflow, "publish");

    // Write access to packages exists once, in publish, and nowhere else; image-a
    // builds and gates with read access only.
    assert_eq!(
        workflow.matches("packages: write").count(),
        1,
        "`packages: write` must appear exactly once"
    );
    assert!(
        publish.contains("packages: write"),
        "`packages: write` must be in the publish job"
    );
    assert!(
        image_a.contains("permissions:\n      contents: read\n") && !image_a.contains("packages:"),
        "image-a must hold only `contents: read`"
    );
    for other in ["mt", "image-a", "image-b", "compare"] {
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

    // The login is SHA-pinned (checked for every `uses:` by the SHA test) and
    // happens in publish only.
    assert_eq!(workflow.matches("docker/login-action@").count(), 1);
    assert!(publish.contains("docker/login-action@"));
    assert!(!image_a.contains("docker/login-action@") && !image_a.contains("docker push"));
    assert!(
        !workflow.contains("github.event_name != 'pull_request'"),
        "publishing must not be gated on `!= pull_request` (any branch, schedule, dispatch)"
    );

    // publish runs after every gate, for pushes to main only (WR-09), one at a
    // time per commit, and neither builds nor checks out anything.
    assert!(publish.contains("needs: [image-a, image-b, compare]"));
    assert!(
        publish.contains(PUBLISH_GUARD),
        "publish must carry the job-level publish guard"
    );
    assert!(
        publish.contains("group: container-publish-${{ github.sha }}")
            && publish.contains("cancel-in-progress: false"),
        "publish must serialize per commit and never be cancelled"
    );
    for forbidden in ["buildx build", "docker build", "actions/checkout"] {
        assert!(
            !publish.contains(forbidden),
            "publish must not contain `{forbidden}`: the gated image is never rebuilt"
        );
    }
    // WR-09: `main` is the only ref the workflow names. A branch name (or any
    // `||` alternative) in a guard would let anyone who can push that branch
    // publish into the package.
    assert_eq!(
        workflow.matches("refs/heads/").count(),
        3,
        "only the three publish guards may name a ref"
    );
    assert!(
        !workflow.contains("refs/heads/claude/"),
        "no working branch may hold publish rights"
    );
    assert!(
        !workflow.contains("refs/heads/main' ||") && !workflow.contains("|| github.ref"),
        "the publish guard must not have an alternative ref"
    );
    // The guard: publish's job-level if, image-a's save step and its upload step.
    assert_eq!(
        workflow.matches(PUBLISH_GUARD).count(),
        3,
        "the guard must appear on publish, on image-a's save step and on its archive upload"
    );

    // A newer run may cancel only a pull_request run, never a push that is
    // between the gates and the publish job (WR-04).
    assert!(
        workflow.contains("cancel-in-progress: ${{ github.event_name == 'pull_request' }}"),
        "cancel-in-progress must be limited to pull_request runs"
    );
    assert!(!workflow.contains("cancel-in-progress: true"));

    // image-a saves exactly the image its gates passed, after every gate, and the
    // job outputs carry its id and archive hash to publish.
    assert!(image_a.contains("docker save"));
    let order = positions(
        image_a,
        "image-a",
        &[
            "pin-binding",
            "annotate notice check-a",
            "annotate notice negative-pin",
            "annotate notice manifest-a",
            "annotate notice smoke",
            "id: save",
        ],
    );
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "the save step must come after the pin-binding, check-a, negative-pin, manifest-a and smoke steps: {order:?}"
    );
    let save = image_a.find("id: save").unwrap();
    let save_run = save + image_a[save..].find("run:").expect("save has a run");
    assert!(
        image_a[save..save_run].contains(PUBLISH_GUARD)
            && !image_a[save..save_run].contains("always()")
            && !image_a[save..save_run].contains("!cancelled()"),
        "the save step must carry the guard and no status function"
    );
    assert!(image_a.contains("image_id: ${{ steps.save.outputs.image_id }}"));
    assert!(image_a.contains("archive_sha256: ${{ steps.save.outputs.archive_sha256 }}"));
    assert!(!image_a.contains("image_ref"));
    assert!(image_a.contains("annotate notice image-archive"));
    assert!(!image_a.contains("annotate notice image-digest"));
    assert!(!image_a.contains("annotate notice verified-image"));
    // The archive travels as an artifact with the same SHA-pinned actions.
    let upload = "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9";
    let download = "actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333";
    assert!(image_a.contains("name: image-a-image"));
    assert!(image_a.contains(upload));
    assert!(publish.contains("name: image-a-image"));
    assert!(publish.contains(download));

    // publish checks the archive hash before loading it, the image id after, and
    // only then logs in; it looks the tag up before the push, pushes, and announces.
    let order = positions(
        publish,
        "publish",
        &[
            "sha256sum -c",
            "docker load",
            "{{.Id}}",
            "docker/login-action@",
            "docker manifest inspect",
            "docker push",
            "annotate notice verified-image",
        ],
    );
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "publish steps are out of order: {order:?}"
    );
    assert!(publish.contains("needs.image-a.outputs.archive_sha256"));
    assert!(publish.contains("needs.image-a.outputs.image_id"));
    assert!(
        !publish[order[0]..order[1]].contains("docker/login-action@"),
        "nothing may log in before the archive is checked"
    );
    // The existing-tag lookup fails closed, and the push and announcement follow it.
    assert!(publish.contains("id: existing"));
    let lookup = publish.find("docker manifest inspect").unwrap();
    let lookup_end = publish[lookup..]
        .find("- name:")
        .map_or(publish.len(), |n| lookup + n);
    let lookup_step = &publish[lookup..lookup_end];
    for needle in [
        "exists=true",
        "exists=false",
        "annotate warning publish-skipped",
        "no such manifest|manifest unknown|not found",
        "exit 1",
    ] {
        assert!(
            lookup_step.contains(needle),
            "the tag lookup lacks `{needle}`"
        );
    }
    assert_eq!(
        publish
            .matches("if: ${{ steps.existing.outputs.exists == 'false' }}")
            .count(),
        2,
        "the push and the announcement run only when the tag is absent"
    );
    assert!(publish.contains("ghcr.io/sat-wik/migrationtool-toolchain"));
    assert!(publish.contains("{{index .RepoDigests 0}}"));
    assert!(
        publish.contains(r"^ghcr\.io/sat-wik/migrationtool-toolchain@sha256:[0-9a-f]{64}$"),
        "publish must validate the digest reference it announces"
    );
    assert!(publish.contains("annotate notice image-digest"));
    assert!(!publish[publish.find("docker push").unwrap()..].contains("docker build"));

    // verified-image is announced once, in publish, with the compare job's manifest
    // hash, after the push (WR-04, CONTEXT D-24).
    assert_eq!(
        workflow.matches("annotate notice verified-image").count(),
        1
    );
    assert!(publish.contains("needs.compare.outputs.manifest_sha256"));
    let compare = workflow_job_text(&workflow, "compare");
    assert!(!compare.contains("IMAGE_REF") && !compare.contains("verified-image"));
    let diff = compare
        .find("manifest-diff")
        .expect("compare runs manifest-diff");
    assert!(
        compare[diff..].contains("exit 1"),
        "a manifest difference must fail the job"
    );
    assert!(
        compare.contains("manifest_sha256=") && compare.contains("id: diff"),
        "compare must export the manifest hash"
    );
    assert!(compare.contains("manifest_sha256: ${{ steps.diff.outputs.manifest_sha256 }}"));
}

#[test]
fn tool_03_dockerfile_asserts_the_rustup_update_hash_comes_from_the_pins() {
    // WR-07: the build fails when the update-hash rustup stored for a pinned
    // toolchain is not the start of the pin it was given.
    let dockerfile = read_repo_file("container/Dockerfile");
    let rustup_run = dockerfile_instructions(&dockerfile)
        .into_iter()
        .find(|(keyword, rest)| keyword == "RUN" && rest.contains("rustup toolchain install"))
        .map(|(_, rest)| rest)
        .expect("a RUN installs the toolchains");
    let install = rustup_run
        .rfind("rustup toolchain install")
        .expect("install present");
    let after = &rustup_run[install..];
    assert!(
        after.contains("update-hashes/"),
        "the install RUN must read ${{RUSTUP_HOME}}/update-hashes after installing"
    );
    for pin in ["RUST_TOOL_CHANNEL_SHA256", "RUST_BITCODE_CHANNEL_SHA256"] {
        assert!(
            after.contains(&format!("${{{pin}}}")),
            "the update-hash check must compare against {pin}"
        );
    }
    let case = after
        .find("case \"${sha}\" in")
        .expect("the update-hash is compared with the pin");
    assert!(
        after[case..].contains("exit 1"),
        "a differing update-hash must fail the build"
    );
    assert!(
        after
            .find("rm -rf /tmp/rust-dist")
            .is_some_and(|rm| rm > case),
        "the check must run while the mirror still exists"
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

    // WR-07: the gate also measures the installed result. Cargo's own install
    // ledger must name each pinned crate with the unpacked, checked directory as
    // its source, and the container script must fail on any read error so that a
    // missing file cannot pass as an empty section.
    assert!(
        image_a.contains("/opt/cargo-tools/.crates2.json"),
        "image-a must read cargo's install ledger .crates2.json"
    );
    assert!(
        image_a.contains("(path+file:///tmp/crates/${name}-${version})"),
        "image-a must require the path+file:///tmp/crates/<name>-<version> source"
    );
    assert!(
        image_a.contains("BIND'\n          set -eu\n"),
        "the binding script must run with `set -eu`"
    );
    // The update-hash is a witness derived from the Dockerfile's own sidecar and
    // the notice must not present it as independent corroboration.
    assert!(
        image_a.contains("a witness, not the binding"),
        "the update-hash must be labelled a witness"
    );
    assert!(
        !image_a.contains("is a prefix of the pin"),
        "the tautological update-hash wording must be gone"
    );

    // The gate comes before the image is saved for publishing, and the save step
    // carries no status function, so a failed gate keeps the image out of GHCR;
    // publish needs image-a.
    let first_gate = image_a
        .find("pin-binding")
        .expect("image-a mentions pin-binding");
    let save = image_a
        .find("id: save")
        .expect("the save step must have the id `save`");
    assert!(
        first_gate < save,
        "the pin-binding gate must come before the save step"
    );
    let publish = workflow_job_text(&workflow, "publish");
    assert!(
        publish.contains("needs: [image-a, image-b, compare]"),
        "publish must need image-a"
    );
}

#[test]
fn tool_03_workflow_gates_each_recorded_archive_on_the_pinned_manifest() {
    // WR-07: the gate also binds every archive line of the record to a pair of the
    // pin-verified manifest, using in-image copies whose sha256 it compares with
    // the pins itself, and fails on any record line it cannot classify.
    let workflow = read_repo_file(".github/workflows/container.yml");
    let image_a = workflow_job_text(&workflow, "image-a");
    for needle in [
        "/opt/cargo-tools/share/mt/channel-rust-",
        "sha256sum \"$copy\"",
        "xz_hash",
        "xz_url",
        "OK archive",
        "MISMATCH archive",
        "unclassified record line",
        "--network none --user 65534:65534 --read-only mt-toolchain:ci",
    ] {
        assert!(
            image_a.contains(needle),
            "the pin-binding gate lacks `{needle}`"
        );
    }
    // The copies are compared with the pins read from image-args.txt.
    let check = image_a
        .find("check_archives()")
        .expect("the gate defines check_archives");
    let body = &image_a[check..];
    assert!(
        body.find("!= \"$pin\"").is_some() && body.contains("MISMATCH"),
        "check_archives must compare the copy's sha256 with the pin"
    );
    for call in [
        "check_archives \"$tool_version\" \"$tool_pin\"",
        "check_archives \"$bitcode_version\" \"$bitcode_pin\"",
        "classify_record",
    ] {
        assert!(image_a.contains(call), "the gate never runs `{call}`");
    }
    // The assertions of the existing pin-binding test still hold.
    assert!(image_a.contains("annotate error pin-binding"));
}
