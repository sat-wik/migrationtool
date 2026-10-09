#![allow(clippy::unwrap_used, clippy::expect_used)]
//! TOOL-02 and TOOL-03 (static half): the full pins schema, its validation,
//! the build-argument rendering and the tool-output parsers. Everything here
//! runs in plain `cargo test`: no Docker, no network, no external process.

use std::collections::BTreeMap;

use mt_toolchain::build_args::{self, Scope};
use mt_toolchain::pins::{LlvmSource, Pins, ToolPinStatus};
use mt_toolchain::version::{self, DpkgPackage, LlvmVersion};
use serde::Deserialize;

const FULL: &str = include_str!("fixtures/pins-full.toml");
const OUTPUTS: &str = include_str!("fixtures/tool-outputs.toml");

/// One recorded tool invocation: where the text came from and what it printed.
#[derive(Debug, Deserialize)]
struct Recorded {
    source: String,
    stdout: String,
    stderr: String,
}

fn outputs() -> BTreeMap<String, Recorded> {
    toml::from_str(OUTPUTS).expect("tool-outputs.toml parses")
}

/// The first `group` match in stdout, then stderr (the order `mt toolchain check` uses).
fn first_match(re: &regex::Regex, group: &str, recorded: &Recorded) -> Option<String> {
    version::extract(re, group, &recorded.stdout)
        .or_else(|| version::extract(re, group, &recorded.stderr))
}

/// Replace `from` with `to` in the full fixture, asserting `from` is present.
fn mutate(from: &str, to: &str) -> String {
    assert!(FULL.contains(from), "fixture does not contain {from:?}");
    FULL.replace(from, to)
}

/// Parse then validate; the error text carries every issue or the parse error.
fn outcome(text: &str) -> Result<Pins, String> {
    let pins = Pins::parse(text).map_err(|e| e.to_string())?;
    pins.validate().map_err(|e| e.to_string())?;
    Ok(pins)
}

fn outcome_complete(text: &str) -> Result<Pins, String> {
    let pins = Pins::parse(text).map_err(|e| e.to_string())?;
    pins.validate_complete().map_err(|e| e.to_string())?;
    Ok(pins)
}

/// The full fixture with the LLVM major and every LLVM-versioned name moved to `n`.
fn with_llvm_major(n: u32) -> String {
    FULL.replace("\nmajor = 16\n", &format!("\nmajor = {n}\n"))
        .replace("llvm-16", &format!("llvm-{n}"))
        .replace("clang-16", &format!("clang-{n}"))
        .replace("clang-cpp16", &format!("clang-cpp{n}"))
}

/// Remove the text from `start` (inclusive) up to `end` (exclusive).
fn remove_between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).expect("start marker present");
    let to = from + text[from..].find(end).expect("end marker present");
    format!("{}{}", &text[..from], &text[to..])
}

fn assert_fails_naming(text: &str, needle: &str) {
    match outcome(text) {
        Ok(_) => panic!("expected validation to fail naming {needle:?}"),
        Err(message) => assert!(
            message.contains(needle),
            "message does not name {needle:?}: {message}"
        ),
    }
}

#[test]
fn tool_03_pins_digests_and_commits_are_well_formed() {
    outcome(FULL).expect("the full fixture is valid");

    let cases: [(String, &str); 8] = [
        (
            mutate(
                "dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71",
                "dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb7",
            ),
            "source.rustup.sha256",
        ),
        (
            mutate(
                "8f6903f8ceb084d9227b9ef991490413014d991874a1e34074443c2a72b14dbd",
                "8F6903F8CEB084D9227B9EF991490413014D991874A1E34074443C2A72B14DBD",
            ),
            "source.arm_gnu.sha256",
        ),
        (
            mutate(
                "92ee8201a050184aacaaeef645ade491f5c93c41",
                "92ee8201a050184aacaaeef645ade491f5c93c4",
            ),
            "source.klee.commit",
        ),
        (
            mutate(
                "sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587",
                "7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587",
            ),
            "image.base_digest",
        ),
        (
            mutate(
                "@sha256:b059f8d7226d0b326bc871af489a5dddf65e36af4d20d14251b072974d9ee87c",
                "",
            ),
            "ci.buildkit_image",
        ),
        (
            mutate("https://developer.arm.com", "http://developer.arm.com"),
            "source.arm_gnu.url",
        ),
        (
            mutate("platform = \"linux/amd64\"", "platform = \"linux/arm64\""),
            "image.platform",
        ),
        (
            mutate("20261005T000000Z", "2026-10-05"),
            "image.snapshot_timestamp",
        ),
    ];
    for (text, needle) in &cases {
        assert_fails_naming(text, needle);
    }
}

#[test]
fn tool_03_pins_reject_placeholders_and_unknown_fields() {
    for bad in [
        "",
        "TODO",
        "tbd",
        "‹discover›",
        "FIXME later",
        "placeholder",
    ] {
        let text = mutate(
            "base = \"debian:bookworm-20261005-slim\"",
            &format!("base = \"{bad}\""),
        );
        assert_fails_naming(&text, "image.base");
        assert_fails_naming(&text, "placeholder");
    }

    // A word that merely contains a marker is not a placeholder.
    outcome(&mutate(
        "base = \"debian:bookworm-20261005-slim\"",
        "base = \"mastodon:bookworm\"",
    ))
    .expect("a word containing 'todo' is not a placeholder");

    // Regex fields are exempt from the placeholder rule.
    outcome(&mutate(
        "version_regex = 'c2rust (?P<version>\\d+\\.\\d+\\.\\d+)'",
        "version_regex = 'c2rust TODO (?P<version>\\d+\\.\\d+\\.\\d+)'",
    ))
    .expect("regex fields may contain marker words");

    let unknown: [(&str, &str); 8] = [
        ("[image]\n", "[image]\nbogus = 1\n"),
        ("[llvm]\n", "[llvm]\nbogus = 1\n"),
        ("[rust.tool]\n", "[rust.tool]\nbogus = 1\n"),
        ("[source.klee]\n", "[source.klee]\nbogus = 1\n"),
        ("[ci]\n", "[ci]\nbogus = 1\n"),
        ("[tool.clang]\n", "[tool.clang]\nbogus = 1\n"),
        ("[tool.clang.llvm]\n", "[tool.clang.llvm]\nbogus = 1\n"),
        ("schema_version = 1\n", "schema_version = 1\nbogus = 1\n"),
    ];
    for (from, to) in unknown {
        let message = outcome(&mutate(from, to)).expect_err("unknown key must be rejected");
        assert!(message.contains("bogus"), "message: {message}");
    }
}

#[test]
fn tool_02_pins_llvm_major_in_supported_range_and_provisional() {
    for ok in [16, 19] {
        outcome(&with_llvm_major(ok)).unwrap_or_else(|e| panic!("major {ok} must pass: {e}"));
    }
    for bad in [15, 20] {
        assert_fails_naming(&with_llvm_major(bad), "llvm.major");
    }

    for bad in [18, 20] {
        assert_fails_naming(
            &mutate("max_major = 19", &format!("max_major = {bad}")),
            "llvm.max_major",
        );
    }

    outcome(&mutate("status = \"provisional\"", "status = \"final\""))
        .expect("final is an accepted status");
    let message = outcome(&mutate("status = \"provisional\"", "status = \"draft\""))
        .expect_err("an unknown status must be rejected");
    assert!(message.contains("draft"), "message: {message}");

    assert_fails_naming(
        &mutate(
            "decision = \"D-15\"\n\n[rust]",
            "decision = \"15\"\n\n[rust]",
        ),
        "llvm.decision",
    );
}

#[test]
fn tool_02_pins_llvm_major_tokens_are_consistent() {
    outcome(FULL).expect("the full fixture is consistent");

    // Only the major moves: tool_path and the other names still say 16.
    let message = outcome(&mutate("\nmajor = 16\n", "\nmajor = 17\n"))
        .expect_err("a stale llvm-16 token must be rejected");
    assert!(message.contains("image.tool_path"), "message: {message}");
    assert!(message.contains("llvm-16"), "message: {message}");

    // Every token moves with the major.
    outcome(&with_llvm_major(17)).expect("a fully re-tokened pins file is consistent");

    // Single stale tokens in each place a token may live.
    assert_fails_naming(
        &mutate("\"clang-16\" = ", "\"clang-15\" = "),
        "apt.clang-15",
    );
    assert_fails_naming(
        &mutate(
            "bin = \"/usr/lib/llvm-16/bin/clang\"",
            "bin = \"/usr/lib/llvm-15/bin/clang\"",
        ),
        "tool.clang.bin",
    );
    assert_fails_naming(
        &mutate("\"libclang-cpp16-dev\" = ", "\"libclang-cpp15-dev\" = "),
        "apt.libclang-cpp15-dev",
    );
}

#[test]
fn tool_03_pins_completeness_requires_every_container_tool() {
    outcome_complete(FULL).expect("the full fixture is complete");

    // A tool table missing entirely.
    let no_qemu_arm = remove_between(FULL, "[tool.qemu_arm]", "[tool.qemu_system_arm]");
    outcome(&no_qemu_arm).expect("plain validation does not require completeness");
    let message = outcome_complete(&no_qemu_arm).expect_err("qemu_arm is required");
    assert!(message.contains("tool.qemu_arm"), "message: {message}");

    // Kani must stay not_installed (CONTEXT D-10).
    let kani_installed = mutate(
        "[tool.kani]\nstatus = \"not_installed\"\nreason = \"deferred to v2.0\"\n",
        "[tool.kani]\nbin = \"/opt/kani/bin/kani\"\nversion_args = [\"--version\"]\nversion_regex = 'kani (?P<version>\\S+)'\nexpect = \"0.1.0\"\n",
    );
    outcome(&kani_installed).expect("an installed kani is a valid entry");
    let message = outcome_complete(&kani_installed).expect_err("kani must be not_installed");
    assert!(message.contains("tool.kani"), "message: {message}");

    // Hayroll listed nowhere.
    let no_hayroll = remove_between(FULL, "[tool.hayroll]", "[tool.kani]");
    let message = outcome_complete(&no_hayroll).expect_err("hayroll must be listed");
    assert!(message.contains("tool.hayroll"), "message: {message}");

    // klee without its LLVM rule.
    let no_klee_rule = remove_between(FULL, "[tool.klee.llvm]", "[tool.llvm_config]");
    outcome(&no_klee_rule).expect("plain validation does not require an LLVM rule");
    let message = outcome_complete(&no_klee_rule).expect_err("klee needs an LLVM rule");
    assert!(message.contains("tool.klee.llvm"), "message: {message}");

    // Empty [apt].
    let no_apt = remove_between(FULL, "[apt]\n", "[ci]");
    outcome(&no_apt).expect("plain validation allows an empty [apt]");
    let message = outcome_complete(&no_apt).expect_err("apt must not be empty");
    assert!(message.contains("apt"), "message: {message}");
}

#[test]
fn tool_03_build_args_render_sorted_lines_per_scope() {
    let pins = outcome(FULL).unwrap();

    let lines = build_args::render_lines(&pins, Scope::Image).unwrap();
    let names: Vec<&str> = lines
        .lines()
        .map(|line| line.split_once('=').expect("NAME=value").0)
        .collect();
    let mut expected: Vec<&str> = vec![
        "ARM_GNU_SHA256",
        "ARM_GNU_URL",
        "ARM_GNU_VERSION",
        "BASE_DIGEST",
        "BASE_REF",
        "C2RUST_NAME",
        "C2RUST_SHA256",
        "C2RUST_VERSION",
        "CARGO_HOME",
        "CARGO_MUTANTS_NAME",
        "CARGO_MUTANTS_SHA256",
        "CARGO_MUTANTS_VERSION",
        "KLEE_COMMIT",
        "KLEE_TAG",
        "KLEE_UCLIBC_COMMIT",
        "KLEE_UCLIBC_TAG",
        "KLEE_UCLIBC_URL",
        "KLEE_URL",
        "LLVM_MAJOR",
        "PLATFORM",
        "RUSTUP_HOME",
        "RUSTUP_SHA256",
        "RUSTUP_URL",
        "RUSTUP_VERSION",
        "RUST_BITCODE_CHANNEL_SHA256",
        "RUST_BITCODE_COMMIT",
        "RUST_BITCODE_VERSION",
        "RUST_HOST",
        "RUST_TOOL_CHANNEL_SHA256",
        "RUST_TOOL_COMMIT",
        "RUST_TOOL_COMPONENTS",
        "RUST_TOOL_TARGETS",
        "RUST_TOOL_VERSION",
        "SNAPSHOT_TIMESTAMP",
        "SOURCE_DATE_EPOCH",
        "TINYCBOR_COMMIT",
        "TINYCBOR_PARENT",
        "TOOL_PATH",
    ];
    expected.sort_unstable();
    assert_eq!(
        names, expected,
        "image build args must be exactly this sorted set"
    );
    assert!(lines.starts_with("ARM_GNU_SHA256="), "lines: {lines}");
    assert!(lines.ends_with('\n'));

    let args = build_args::render(&pins, Scope::Image).unwrap();
    assert_eq!(args["LLVM_MAJOR"], "16");
    assert_eq!(args["BASE_REF"], "debian:bookworm-20261005-slim");
    assert_eq!(
        args["RUST_TOOL_TARGETS"],
        "thumbv7em-none-eabihf,x86_64-unknown-linux-musl"
    );
    assert_eq!(args["RUST_TOOL_COMPONENTS"], "clippy,rustfmt");
    assert_eq!(args["TINYCBOR_PARENT"], "c2rust");
    assert_eq!(args["C2RUST_NAME"], "c2rust");
    assert_eq!(
        args["KLEE_COMMIT"],
        "92ee8201a050184aacaaeef645ade491f5c93c41"
    );
    assert!(args.keys().all(|k| !k.starts_with("APT_")), "no apt keys");
    assert!(!args.contains_key("DPKG_QUERY"));
    assert!(!args.contains_key("LLVM_MAX_MAJOR"));

    // Rendering is deterministic.
    assert_eq!(
        lines,
        build_args::render_lines(&pins, Scope::Image).unwrap()
    );

    let ci = build_args::render_lines(&pins, Scope::Ci).unwrap();
    let ci_lines: Vec<&str> = ci.lines().collect();
    assert_eq!(ci_lines.len(), 2, "ci scope: {ci}");
    assert!(ci_lines[0].starts_with("BUILDKIT_IMAGE=moby/buildkit:v0.34.0@sha256:"));
    assert_eq!(ci_lines[1], "CARGO_DENY_VERSION=0.20.2");
}

#[test]
fn tool_03_build_args_reject_values_with_whitespace() {
    let spaced = Pins::parse(&mutate(
        "tool_path = \"/usr/lib/llvm-16/bin:",
        "tool_path = \"/usr/lib/llvm-16/bin /extra:",
    ))
    .unwrap();
    let err = build_args::render_lines(&spaced, Scope::Image).expect_err("space must be rejected");
    assert!(err.to_string().contains("TOOL_PATH"), "error: {err}");

    // A newline and a NUL, written as TOML escapes inside the URL.
    for escape in ["\\n", "\\u0000", "\\t"] {
        let pins = Pins::parse(&mutate(
            "/binrel/arm-gnu-toolchain",
            &format!("/binrel/{escape}arm-gnu-toolchain"),
        ))
        .unwrap();
        let err = build_args::render(&pins, Scope::Image)
            .expect_err("control and whitespace characters must be rejected");
        assert!(err.to_string().contains("ARM_GNU_URL"), "error: {err}");
    }

    // The ci scope checks its own values.
    let ci = Pins::parse(&mutate(
        "cargo_deny_version = \"0.20.2\"",
        "cargo_deny_version = \"0.20.2 \"",
    ))
    .unwrap();
    let err = build_args::render(&ci, Scope::Ci).expect_err("trailing space must be rejected");
    assert!(
        err.to_string().contains("CARGO_DENY_VERSION"),
        "error: {err}"
    );

    // A clean file still renders.
    let clean = Pins::parse(FULL).unwrap();
    build_args::render(&clean, Scope::Image).unwrap();
}

#[test]
fn tool_02_parses_llvm_major_from_tool_outputs() {
    let pins = Pins::parse(FULL).unwrap();
    let recorded = outputs();

    // clang and llvm-config: the major is the first component of the version.
    for key in ["clang", "llvm_config"] {
        let spec = &pins.tool[key];
        let re = version::compile_pattern(&spec.version_regex, "version").unwrap();
        let found = first_match(&re, "version", &recorded[key]).expect("version line");
        assert_eq!(
            version::llvm_major_from_version(&found),
            Some(16),
            "{key}: {found}"
        );
    }

    // klee: the LLVM version line; the `Host CPU:` line is ignored.
    let klee = &pins.tool["klee"];
    let rule = klee.llvm.as_ref().expect("klee has an LLVM rule");
    assert_eq!(rule.source, LlvmSource::Output);
    let re = version::compile_pattern(rule.regex.as_deref().unwrap(), "major").unwrap();
    let major = first_match(&re, "major", &recorded["klee"]).expect("LLVM line");
    assert_eq!(major, "16");
    assert!(recorded["klee"].stdout.contains("Host CPU:"));

    // c2rust: the ldd probe.
    let rule = pins.tool["c2rust"]
        .llvm
        .as_ref()
        .expect("c2rust has a rule");
    assert_eq!(rule.source, LlvmSource::Probe);
    let re = version::compile_pattern(rule.regex.as_deref().unwrap(), "major").unwrap();
    assert_eq!(
        first_match(&re, "major", &recorded["c2rust_ldd"]).as_deref(),
        Some("16")
    );

    // The two rustc toolchains differ: the tool rustc must never be the bitcode compiler.
    let tool = version::parse_rustc_vv(&recorded["rustc_tool"].stdout).expect("rustc_tool");
    assert_eq!(tool.llvm.map(|v| v.major), Some(23));
    let bitcode =
        version::parse_rustc_vv(&recorded["rustc_bitcode"].stdout).expect("rustc_bitcode");
    assert_eq!(bitcode.llvm.map(|v| v.major), Some(16));
}

#[test]
fn tool_02_parses_rustc_vv_release_commit_and_llvm() {
    let text = "rustc 1.72.1 (d5c2e9c34 2023-09-13)\nbinary: rustc\ncommit-hash: d5c2e9c342b358556da91d61ed4133f6f50fc0c3\ncommit-date: 2023-09-13\nhost: x86_64-unknown-linux-gnu\nrelease: 1.72.1\nLLVM version: 16.0.5\n";
    let parsed = version::parse_rustc_vv(text).expect("parses");
    assert_eq!(parsed.release, "1.72.1");
    assert_eq!(
        parsed.commit_hash,
        "d5c2e9c342b358556da91d61ed4133f6f50fc0c3"
    );
    let llvm = parsed.llvm.expect("LLVM line present");
    assert_eq!(
        llvm,
        LlvmVersion {
            major: 16,
            minor: 0,
            patch: 5
        }
    );
    assert_eq!(llvm.to_string(), "16.0.5");

    // The captured fixture agrees with the literal text.
    let captured = version::parse_rustc_vv(&outputs()["rustc_bitcode"].stdout).unwrap();
    assert_eq!(captured, parsed);

    // No LLVM line (a rustc built without LLVM information) still parses.
    let without = "rustc 1.72.1 (d5c2e9c34 2023-09-13)\nbinary: rustc\ncommit-hash: d5c2e9c342b358556da91d61ed4133f6f50fc0c3\nrelease: 1.72.1\n";
    let parsed = version::parse_rustc_vv(without).expect("parses without LLVM");
    assert_eq!(parsed.llvm, None);
    assert_eq!(parsed.release, "1.72.1");

    // Text that is not `rustc -vV` output is rejected, not guessed at.
    assert_eq!(version::parse_rustc_vv(""), None);
    assert_eq!(version::parse_rustc_vv("command not found\n"), None);
    assert_eq!(
        version::parse_rustc_vv("release: 1.72.1\nLLVM version: sixteen\n"),
        None
    );

    // LLVM triples.
    assert_eq!(
        version::parse_llvm_version("23.1.1").map(|v| v.major),
        Some(23)
    );
    assert_eq!(version::parse_llvm_version("16.0"), None);
    assert_eq!(version::parse_llvm_version("16.0.x"), None);
    assert_eq!(version::parse_llvm_version(""), None);
}

#[test]
fn tool_02_parses_dpkg_query_and_llvm_family_majors() {
    let recorded = outputs();
    let packages = version::parse_dpkg_query(&recorded["dpkg_query"].stdout);
    assert_eq!(packages.len(), 17, "one package per line: {packages:?}");
    assert_eq!(
        packages[1],
        DpkgPackage {
            name: "clang-16".to_owned(),
            version: "1:16.0.6-15~deb12u1".to_owned(),
            status: "installed".to_owned(),
        }
    );

    let majors = version::llvm_family_majors(&packages);
    for name in [
        "clang-16",
        "libclang1-16",
        "libclang-cpp16",
        "libllvm16",
        "libclang-common-16-dev",
        "llvm-16-dev",
    ] {
        assert_eq!(majors.get(name), Some(&16), "{name}: {majors:?}");
    }
    // Not installed, or installed without a major in the name, or not LLVM at all.
    for name in [
        "libllvm18",
        "llvm-runtime",
        "bear",
        "qemu-user",
        "libz3-dev",
    ] {
        assert!(!majors.contains_key(name), "{name} must be ignored");
    }

    // A second installed major is visible to the caller.
    let extra = version::parse_dpkg_query("libllvm18\t1:18.1.8-12\tinstalled\n");
    let mut all = packages;
    all.extend(extra);
    let majors = version::llvm_family_majors(&all);
    assert_eq!(majors.get("libllvm18"), Some(&18));

    // Malformed lines are skipped; blank input is empty.
    assert!(version::parse_dpkg_query("").is_empty());
    assert_eq!(
        version::parse_dpkg_query("no-tabs-here\nbear\t3.1.1-1\tinstalled\n").len(),
        1
    );

    // Debian versions: drop the epoch and the revision.
    assert_eq!(version::upstream_version("1:16.0.6-15~deb12u1"), "16.0.6");
    assert_eq!(
        version::upstream_version("1:7.2+dfsg-7+deb12u18"),
        "7.2+dfsg"
    );
    assert_eq!(version::upstream_version("3.1.1-1"), "3.1.1");
    assert_eq!(version::upstream_version("4.8.12"), "4.8.12");
}

#[test]
fn tool_02_fixture_pins_regexes_parse_fixture_outputs() {
    let pins = Pins::parse(FULL).unwrap();
    let recorded = outputs();

    for (key, entry) in &recorded {
        assert!(!entry.source.trim().is_empty(), "{key}: source is empty");
    }
    assert!(recorded["rustc_bitcode"].source.contains("captured"));
    assert!(
        recorded["rustc_bitcode"]
            .stdout
            .contains("LLVM version: 16.0.5")
    );
    assert!(recorded["rustc_tool"].source.contains("captured"));

    let mut checked = 0;
    for (key, spec) in &pins.tool {
        if spec.status == ToolPinStatus::NotInstalled {
            continue;
        }
        let output = recorded
            .get(key)
            .unwrap_or_else(|| panic!("no fixture output for tool {key}"));
        let re = version::compile_pattern(&spec.version_regex, "version")
            .unwrap_or_else(|e| panic!("{key}: {e}"));
        let found = first_match(&re, "version", output)
            .unwrap_or_else(|| panic!("{key}: version regex did not match its fixture"));
        assert_eq!(found, spec.expect, "{key}");

        if let Some(rule) = &spec.llvm {
            let major = match rule.source {
                LlvmSource::Version => version::llvm_major_from_version(&found),
                LlvmSource::Output | LlvmSource::Probe => {
                    let text = if rule.source == LlvmSource::Probe {
                        &recorded[&format!("{key}_ldd")]
                    } else {
                        output
                    };
                    let re = version::compile_pattern(rule.regex.as_deref().unwrap(), "major")
                        .unwrap_or_else(|e| panic!("{key}: {e}"));
                    first_match(&re, "major", text).and_then(|m| m.parse::<u32>().ok())
                }
            };
            assert_eq!(major, Some(pins.llvm.major), "{key}: LLVM major");
        }
        checked += 1;
    }
    assert_eq!(
        checked, 10,
        "every installed tool in the fixture was checked"
    );
}

#[test]
fn tool_02_compile_pattern_and_extract_edge_cases() {
    // A valid pattern without the required group is rejected, naming the group.
    let err = version::compile_pattern(r"v(\d+)", "version").expect_err("no named group");
    assert!(err.to_string().contains("version"), "error: {err}");

    // A syntax error is reported, not unwrapped.
    assert!(version::compile_pattern("(unclosed", "version").is_err());

    // A pattern whose compiled form exceeds the 1 MiB limit is rejected (T-01-10).
    assert!(
        version::compile_pattern(r"(?P<version>\w{1000}){1000}", "version").is_err(),
        "an oversized regex must be refused"
    );

    // extract returns the group, or None when nothing matches.
    let re = version::compile_pattern(r"v(?P<version>\d+\.\d+)", "version").unwrap();
    assert_eq!(
        version::extract(&re, "version", "tool v3.2 ready").as_deref(),
        Some("3.2")
    );
    assert_eq!(version::extract(&re, "version", "no version here"), None);
    assert_eq!(version::extract(&re, "other", "tool v3.2"), None);

    // The first dot component, and nothing else, is the major.
    assert_eq!(version::llvm_major_from_version("16.0.6"), Some(16));
    assert_eq!(version::llvm_major_from_version("3.2"), Some(3));
    assert_eq!(version::llvm_major_from_version("16"), Some(16));
    assert_eq!(version::llvm_major_from_version(""), None);
    assert_eq!(version::llvm_major_from_version("x.1"), None);
}
