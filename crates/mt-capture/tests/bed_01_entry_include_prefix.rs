#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(unix)]
//! BED-01 entry check: the probe reproduces the defining translation unit's
//! include prefix (CONTEXT D-23, D-24; gap G-02-1, review IN-07).
//!
//! The probe used to hold `#include "<header>"` alone. A header that relies on
//! what its translation unit includes first (isotp.h uses `uint8_t` and only
//! includes <stdint.h> under `__cplusplus`) then failed to parse, and the whole
//! target was lost to a harness limit and not a defect. The probe now holds the
//! `#include`, `#define` and `#undef` lines the translation unit has before the
//! line that includes the header, and then the header.
//!
//! The prefix is never guessed: a conditional, any other directive or any C
//! token before the header makes it not reproducible, and the probe falls back to
//! the header alone. The comparison itself (exact name, signature after
//! whitespace collapse, header among the `-MM` dependencies) is not touched.
//!
//! The scanner tests are pure; the capture-level tests run clang and the Arm
//! build on the host and skip themselves, visibly, when the host lacks the tools
//! (`support::local_arm_tools`).

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use mt_capture::capture::{CaptureContext, capture};
use mt_capture::entries::{
    EntryError, EntryResult, IncludePrefix, include_prefix, probe_source, probe_source_with_prefix,
    unit_include_prefix,
};
use mt_capture::fetch::fetch;
use mt_capture::manifest::{BuildArgs, BuildSystem, Config, Entry, EntryForm, Target};
use mt_capture::report::{CaptureReport, CaptureVerdict};
use support::{
    Upstream, capture_context, fetch_context, git_target, local_arm_tools, make_upstream,
    runner_config,
};

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// A source tree under a temporary directory.
struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in files {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        Self { dir }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    /// The prefix of `tu_text` as if it were `lib/needs.c` including
    /// `lib/needs.h`, with the tree root as the compile directory.
    fn prefix(&self, tu_text: &str, flags: &[&str]) -> IncludePrefix {
        include_prefix(
            tu_text.as_bytes(),
            &self.path("lib/needs.c"),
            &self.path("lib/needs.h"),
            self.dir.path(),
            &strings(flags),
        )
    }
}

fn base_tree() -> Tree {
    Tree::new(&[
        ("lib/needs.c", ""),
        ("lib/needs.h", ""),
        ("lib/local.h", ""),
        ("inc/shared.h", ""),
    ])
}

fn not_reproducible(got: IncludePrefix) -> String {
    match got {
        IncludePrefix::NotReproducible(reason) => reason,
        other => panic!("expected NotReproducible, got {other:?}"),
    }
}

// ---- the scanner ----------------------------------------------------------

#[test]
fn bed_01_include_prefix_reproduces_includes_and_defines() {
    let tree = base_tree();
    let root = tree.dir.path().to_str().unwrap().to_owned();
    let tu = "\
/* #include \"ghost.h\" is only a comment */
// so is this line
#include <stdint.h>
#define LIMIT 10\\
00   // a trailing comment is removed

#undef LIMIT
#include \"local.h\"
#include \"shared.h\"
#include \"assert.h\"
/* a block comment
   over two lines */
#include \"needs.h\"
int never_reached;
#if 0
";
    let expected = IncludePrefix::Reproduced(vec![
        "#include <stdint.h>".to_owned(),
        "#define LIMIT 1000".to_owned(),
        "#undef LIMIT".to_owned(),
        format!("#include \"{root}/lib/local.h\""),
        format!("#include \"{root}/inc/shared.h\""),
        "#include <assert.h>".to_owned(),
    ]);

    // `-I inc` and `-Iinc` are the same operand; `inc` is relative to the unit's
    // directory, not to the translation unit's.
    assert_eq!(tree.prefix(tu, &["-I", "inc"]), expected);
    assert_eq!(tree.prefix(tu, &["-Iinc"]), expected);
    // A -iquote directory is searched for a quoted include too.
    assert_eq!(tree.prefix(tu, &["-iquote", "inc"]), expected);

    // Without the directory the quoted include is found nowhere and is written
    // in angle form, like "assert.h" (the compiler continues in the system
    // directories).
    let without = tree.prefix(tu, &[]);
    let IncludePrefix::Reproduced(lines) = without else {
        panic!("expected Reproduced, got {without:?}");
    };
    assert_eq!(lines[4], "#include <shared.h>");
    assert_eq!(lines[3], format!("#include \"{root}/lib/local.h\""));

    // An angle include that resolves to nothing stays as written, and one that
    // resolves to the header through -I ends the scan.
    assert_eq!(
        tree.prefix(
            "#include <stdint.h>\n#include <needs.h>\nint x;\n",
            &["-I", "lib"]
        ),
        IncludePrefix::Reproduced(vec!["#include <stdint.h>".to_owned()])
    );
}

#[test]
fn bed_01_include_prefix_refuses_what_it_cannot_reproduce() {
    let tree = base_tree();
    let refused = [
        ("#if 1\n#endif\n", 1),
        ("#ifdef X\n#endif\n", 1),
        ("\n#ifndef X\n#endif\n", 2),
        ("#elif 1\n", 1),
        ("#else\n", 1),
        ("#endif\n", 1),
        ("#pragma once\n", 1),
        ("#error stop\n", 1),
        ("#line 7\n", 1),
        ("#include_next <a.h>\n", 1),
        ("#include NAME\n", 1),
        ("int x;\n", 1),
        ("typedef int t;\n", 1),
        ("\n\n#pragma once\n", 3),
        ("#include <stdint.h>\n#ifdef X\n", 2),
        ("#define A 1\n#if A\n", 2),
        ("/* one\n two */ #pragma once\n", 2),
        ("#if 1 \\\n && 2\n", 1),
    ];
    for (before, line) in refused {
        let reason = not_reproducible(tree.prefix(&format!("{before}#include \"needs.h\"\n"), &[]));
        assert!(
            reason.contains(&format!("line {line}")),
            "{before:?}: {reason}"
        );
    }

    // A translation unit that never includes the header directly.
    for text in ["", "#include <stdint.h>\n", "#include \"local.h\"\n"] {
        not_reproducible(tree.prefix(text, &[]));
    }

    // A resolved path that cannot be written in an #include line.
    for odd in ["we\\ird", "we\"ird", "we\u{7}ird"] {
        let unit = format!("{odd}/needs.c");
        let other = format!("{odd}/x.h");
        let tree = Tree::new(&[
            ("lib/needs.h", ""),
            (unit.as_str(), ""),
            (other.as_str(), ""),
        ]);
        let got = include_prefix(
            b"#include \"x.h\"\n#include \"../lib/needs.h\"\n",
            &tree.path(&unit),
            &tree.path("lib/needs.h"),
            tree.dir.path(),
            &[],
        );
        not_reproducible(got);
    }

    // A translation unit over 8 MiB, one that is missing, and a directory.
    let tree = base_tree();
    let big = tree.path("lib/big.c");
    let file = fs::File::create(&big).unwrap();
    file.set_len(8 * 1024 * 1024 + 1).unwrap();
    let header = tree.path("lib/needs.h");
    for unreadable in [big, tree.path("lib/missing.c"), tree.path("lib")] {
        not_reproducible(unit_include_prefix(
            &unreadable,
            &header,
            tree.dir.path(),
            &[],
        ));
    }
    // A readable unit goes through the same scan.
    fs::write(
        tree.path("lib/needs.c"),
        "#include <stdint.h>\n#include \"needs.h\"\n",
    )
    .unwrap();
    assert_eq!(
        unit_include_prefix(&tree.path("lib/needs.c"), &header, tree.dir.path(), &[]),
        IncludePrefix::Reproduced(vec!["#include <stdint.h>".to_owned()])
    );
}

#[test]
fn bed_01_include_prefix_is_empty_when_the_header_comes_first() {
    let tree = base_tree();
    for (text, flags) in [
        (
            "#include \"needs.h\"\n#include <stdint.h>\nint x;\n#if 1\n",
            &[][..],
        ),
        ("/* c */\n\n// c\n#include \"needs.h\"\n", &[][..]),
        ("#include \"../lib/needs.h\"\n", &[][..]),
        ("#include <needs.h>\n", &["-I", "lib"][..]),
        ("#include <needs.h>\n", &["-isystem", "lib"][..]),
    ] {
        assert_eq!(
            tree.prefix(text, flags),
            IncludePrefix::HeaderFirst,
            "{text:?}"
        );
    }

    // Empty prefix lines give the probe of today, byte for byte.
    let header = Path::new("/work/run/t/default/src/lib/needs.h");
    assert_eq!(
        probe_source_with_prefix(&[], header).unwrap(),
        probe_source(header).unwrap()
    );
    assert_eq!(
        probe_source(header).unwrap(),
        "#include \"/work/run/t/default/src/lib/needs.h\"\n"
    );
}

// ---- capture level --------------------------------------------------------

/// A fetched git target under temporary work and test-bed directories.
struct Bed {
    _work: tempfile::TempDir,
    _testbed: tempfile::TempDir,
    ctx: CaptureContext,
    target: Target,
    _upstream: Upstream,
}

impl Bed {
    fn new(files: &[(&str, &str)]) -> Option<Self> {
        let tools = local_arm_tools()?;
        let upstream = make_upstream(files);
        let work = tempfile::tempdir().unwrap();
        let testbed = tempfile::tempdir().unwrap();
        let ctx = capture_context(work.path(), testbed.path(), tools);
        let mut target = git_target(&upstream, BuildSystem::Make);
        target.build_args = Some(BuildArgs {
            dir: ".".to_owned(),
            goals: vec!["all".to_owned()],
            make_vars: vec!["CC={cc} {cflags}".to_owned()],
            cmake_target: None,
            cmake_options: Vec::new(),
        });
        fetch(
            &runner_config(),
            &fetch_context(work.path()),
            "demo",
            &target,
        )
        .unwrap();
        Some(Self {
            _work: work,
            _testbed: testbed,
            ctx,
            target,
            _upstream: upstream,
        })
    }

    fn capture(&mut self, entries: Vec<Entry>) -> CaptureReport {
        self.target.entry = entries;
        let config = Config {
            name: "default".to_owned(),
            ..Config::default()
        };
        capture(&runner_config(), &self.ctx, "demo", &self.target, &config).unwrap()
    }
}

fn entry(name: &str, header: &str, source: &str, signature: &str) -> Entry {
    Entry {
        name: name.to_owned(),
        header: header.to_owned(),
        source: source.to_owned(),
        form: EntryForm::Direct,
        signature: signature.to_owned(),
        accepted_wrapped: false,
        arg: Vec::new(),
        unobserved: Vec::new(),
    }
}

const NEEDS_H: &str = "#include <stddef.h>\nint needs_parse(const uint8_t *buf, size_t len);\n";
const NEEDS_C: &str = "#include <stdint.h>\n#include \"needs.h\"\n\
                       int needs_parse(const uint8_t *buf, size_t len) { return len ? buf[0] : 0; }\n";
const NEEDS_MAKEFILE: &str = "all: lib/needs.o\nlib/needs.o: lib/needs.c lib/needs.h\n\t$(CC) -c lib/needs.c -o lib/needs.o\n";

fn needs_bed() -> Option<Bed> {
    Bed::new(&[
        ("lib/needs.h", NEEDS_H),
        ("lib/needs.c", NEEDS_C),
        ("Makefile", NEEDS_MAKEFILE),
    ])
}

#[test]
fn bed_01_entry_probe_uses_the_defining_tu_include_prefix() {
    let Some(mut bed) = needs_bed() else { return };

    let report = bed.capture(vec![entry(
        "needs_parse",
        "lib/needs.h",
        "lib/needs.c",
        "int (const uint8_t *,  size_t)",
    )]);

    assert_eq!(
        report.verdict,
        CaptureVerdict::Captured,
        "{:?} {:?}",
        report.causes,
        report.cause_details
    );
    let result = &report.entries["needs_parse"];
    assert!(
        result.found && result.signature_matches && result.header_included,
        "{result:?}"
    );
    assert!(result.passed());
    // The report records the context the probe was built with.
    assert_eq!(result.probe_prefix, ["#include <stdint.h>"]);
}

#[test]
fn bed_01_entry_self_contained_header_still_passes() {
    let header = "#include <stdint.h>\nint sc_parse(const uint8_t *buf, unsigned int len);\n";
    let body = "int sc_parse(const uint8_t *buf, unsigned int len) { return len ? buf[0] : 0; }\n";
    let Some(mut bed) = Bed::new(&[
        ("lib/sc.h", header),
        ("lib/sc_first.c", &format!("#include \"sc.h\"\n{body}")),
        (
            "lib/sc_cond.c",
            &format!("#if 1\n#define SC_MARK 1\n#endif\n#include \"sc.h\"\n{body}"),
        ),
        (
            "Makefile",
            "all: lib/sc_first.o lib/sc_cond.o\n\
             lib/sc_first.o: lib/sc_first.c lib/sc.h\n\t$(CC) -c lib/sc_first.c -o lib/sc_first.o\n\
             lib/sc_cond.o: lib/sc_cond.c lib/sc.h\n\t$(CC) -c lib/sc_cond.c -o lib/sc_cond.o\n",
        ),
    ]) else {
        return;
    };
    let signature = "int (const uint8_t *, unsigned int)";

    // The header comes first in one unit and behind a conditional in the other;
    // both are probed with the header alone, and the report has no prefix.
    for source in ["lib/sc_first.c", "lib/sc_cond.c"] {
        let report = bed.capture(vec![entry("sc_parse", "lib/sc.h", source, signature)]);
        assert_eq!(
            report.verdict,
            CaptureVerdict::Captured,
            "{source}: {:?} {:?}",
            report.causes,
            report.cause_details
        );
        let result = &report.entries["sc_parse"];
        assert!(result.passed(), "{source}: {result:?}");
        assert!(result.probe_prefix.is_empty(), "{source}: {result:?}");
    }

    // When the prefix was not reproducible and the entry fails, the detail says
    // so, so the founder is not left guessing why a context was not used.
    let report = bed.capture(vec![entry(
        "sc_missing",
        "lib/sc.h",
        "lib/sc_cond.c",
        "int (void)",
    )]);
    assert_eq!(report.verdict, CaptureVerdict::CaptureFailed);
    assert_eq!(report.causes, ["entry_mismatch"]);
    let detail = &report.cause_details[0];
    assert!(detail.contains("no function named sc_missing"), "{detail}");
    assert!(
        detail.contains("the defining TU's include prefix could not be reproduced: "),
        "{detail}"
    );
    assert!(detail.contains("line 1"), "{detail}");
}

#[test]
fn bed_01_entry_missing_function_still_fails_with_a_prefix() {
    let Some(mut bed) = needs_bed() else { return };
    let good = entry(
        "needs_parse",
        "lib/needs.h",
        "lib/needs.c",
        "int (const uint8_t *, size_t)",
    );

    // The prefix is reproduced for both, and neither is accepted: a function the
    // header does not declare, and a wrong signature.
    let missing = entry("needs_missing", "lib/needs.h", "lib/needs.c", "int (void)");
    let wrong = entry(
        "needs_parse",
        "lib/needs.h",
        "lib/needs.c",
        "long (const uint8_t *, size_t)",
    );
    for (bad, name, text) in [
        (missing, "needs_missing", "no function named needs_missing"),
        (wrong, "needs_parse", "signature mismatch"),
    ] {
        let report = bed.capture(vec![bad]);
        assert_eq!(report.verdict, CaptureVerdict::CaptureFailed, "{name}");
        assert_eq!(report.causes, ["entry_mismatch"], "{name}");
        assert!(
            report.cause_details[0].contains(text),
            "{:?}",
            report.cause_details
        );
        let result = &report.entries[name];
        assert!(!result.passed(), "{name}: {result:?}");
        assert_eq!(result.probe_prefix, ["#include <stdint.h>"], "{name}");
    }

    // The good entry still passes next to them.
    let report = bed.capture(vec![good]);
    assert_eq!(report.verdict, CaptureVerdict::Captured);
}

// ---- the report -----------------------------------------------------------

#[test]
fn bed_01_entry_probe_prefix_is_recorded_only_when_used() {
    // Header alone: no key, so reports and committed fixtures read as before.
    let alone = serde_json::to_value(EntryResult::default()).unwrap();
    assert!(alone.get("probe_prefix").is_none(), "{alone}");
    let used = EntryResult {
        probe_prefix: strings(&["#include <stdint.h>", "#define A 1"]),
        ..EntryResult::default()
    };
    let value = serde_json::to_value(&used).unwrap();
    assert_eq!(
        value["probe_prefix"],
        serde_json::json!(["#include <stdint.h>", "#define A 1"])
    );
    assert_eq!(serde_json::from_value::<EntryResult>(value).unwrap(), used);

    // A report written before the field existed still parses.
    let old = r#"{"found":true,"signature_matches":true,"header_included":true,
                  "found_signature":null,"detail":null}"#;
    let parsed: EntryResult = serde_json::from_str(old).unwrap();
    assert!(parsed.passed());
    assert!(parsed.probe_prefix.is_empty());

    // The probe file: each prefix line, then the header line probe_source writes.
    let header = Path::new("/w/lib/needs.h");
    assert_eq!(
        probe_source_with_prefix(&strings(&["#include <stdint.h>", "#define A 1"]), header)
            .unwrap(),
        "#include <stdint.h>\n#define A 1\n#include \"/w/lib/needs.h\"\n"
    );
    // A prefix line can never smuggle in a second line, and the header path is
    // checked exactly as before.
    for bad in ["#include <a.h>\n#error x", "#define A\r\n", "#define A\0"] {
        assert!(
            matches!(
                probe_source_with_prefix(&strings(&[bad]), header),
                Err(EntryError::InvalidInput(_))
            ),
            "{bad:?}"
        );
    }
    assert!(probe_source_with_prefix(&[], Path::new("lib/needs.h")).is_err());
    assert!(probe_source_with_prefix(&[], Path::new("/a\"b.h")).is_err());
}
