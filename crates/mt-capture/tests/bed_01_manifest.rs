#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BED-01 and BED-02 (schema half): the manifest parser and validator, driven
//! by a small fixture with made-up values. Everything here runs in plain
//! `cargo test`: no Docker, no network, no external process.

use mt_capture::manifest::{BuildSystem, Category, EntryForm, Manifest, Source};

const SMALL: &str = include_str!("fixtures/manifest-small.toml");

const DEFAULT_MAKE_CONFIG: &str = "[[target.demo_make.config]]\nname = \"default\"\n";
const MAKE_LICENCE: &str = "spdx = \"BSD-3-Clause\"\nelected = \"BSD-3-Clause\"\n";
const MAKE_VARIANT: &str = "name = \"no-float\"\ndefines = [\"DEMO_NO_FLOAT\"]\n";

/// Replace `from` with `to` in the fixture, asserting `from` is present.
fn mutate(from: &str, to: &str) -> String {
    assert!(SMALL.contains(from), "fixture does not contain {from:?}");
    SMALL.replace(from, to)
}

/// Remove the text from `start` (inclusive) up to `end` (exclusive).
fn remove_between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).expect("start marker present");
    let to = from + text[from..].find(end).expect("end marker present");
    format!("{}{}", &text[..from], &text[to..])
}

/// Parse then validate; the error text carries every issue or the parse error.
fn outcome(text: &str) -> Result<Manifest, String> {
    let manifest = Manifest::parse(text).map_err(|e| e.to_string())?;
    manifest.validate().map_err(|e| e.to_string())?;
    Ok(manifest)
}

fn outcome_complete(text: &str) -> Result<Manifest, String> {
    let manifest = Manifest::parse(text).map_err(|e| e.to_string())?;
    manifest.validate_complete().map_err(|e| e.to_string())?;
    Ok(manifest)
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
fn bed_01_small_manifest_parses_and_validates() {
    let manifest = outcome(SMALL).expect("the small fixture is valid");
    outcome_complete(SMALL).expect("the small fixture has a recipe on every target");

    let ids: Vec<&str> = manifest.target.keys().map(String::as_str).collect();
    assert_eq!(ids, ["demo_cmake", "demo_make"]);

    let make = &manifest.target["demo_make"];
    assert_eq!(make.build, BuildSystem::Make);
    assert_eq!(make.category, [Category::Cbor]);
    assert!(matches!(make.source, Source::Git { .. }));
    assert_eq!(make.entry.len(), 2);
    assert_eq!(make.entry[1].form, EntryForm::Range);
    assert_eq!(
        make.default_config().map(|c| c.name.as_str()),
        Some("default")
    );
    assert_eq!(make.variant().map(|c| c.name.as_str()), Some("no-float"));

    let cmake = &manifest.target["demo_cmake"];
    assert_eq!(cmake.build, BuildSystem::Cmake);
    assert_eq!(cmake.category, [Category::Json, Category::Tlv]);
    assert!(matches!(
        cmake.source,
        Source::Archive {
            strip_components: 1,
            ..
        }
    ));
    assert!(cmake.entry[0].accepted_wrapped);
    assert_eq!(
        cmake
            .build_args
            .as_ref()
            .and_then(|b| b.cmake_target.as_deref()),
        Some("demo_json")
    );
}

#[test]
fn bed_01_manifest_rejects_unknown_keys() {
    let cases: [(String, &str); 9] = [
        (
            mutate("schema_version = 1\n", "schema_version = 1\nbogus = 1\n"),
            "top level",
        ),
        (
            mutate(
                "project_decision = \"D-18\"\n",
                "project_decision = \"D-18\"\nbogus = 1\n",
            ),
            "decisions",
        ),
        (mutate("-Os\"]\n", "-Os\"]\nbogus = 1\n"), "baseline"),
        (
            mutate("build = \"make\"\n", "build = \"make\"\nbogus = 1\n"),
            "target",
        ),
        (
            mutate("kind = \"git\"\n", "kind = \"git\"\nbogus = 1\n"),
            "source",
        ),
        (
            mutate("file = \"LICENSE\"\n", "file = \"LICENSE\"\nbogus = 1\n"),
            "licence",
        ),
        (
            mutate(
                "goals = [\"libdemo.a\"]\n",
                "goals = [\"libdemo.a\"]\nbogus = 1\n",
            ),
            "build_args",
        ),
        (
            mutate("form = \"direct\"\n", "form = \"direct\"\nbogus = 1\n"),
            "entry",
        ),
        (
            mutate(
                "defines = [\"DEMO_NO_FLOAT\"]\n",
                "defines = [\"DEMO_NO_FLOAT\"]\nbogus = 1\n",
            ),
            "config",
        ),
    ];
    for (text, level) in cases {
        match outcome(&text) {
            Ok(_) => panic!("an unknown key at the {level} level was accepted"),
            Err(message) => {
                assert!(
                    message.contains("unknown field") && message.contains("bogus"),
                    "the {level} level error does not name the key: {message}"
                );
            }
        }
    }
}

#[test]
fn bed_01_manifest_rejects_duplicate_ids() {
    let text = format!("{SMALL}\n[target.demo_make]\ncategory = [\"cbor\"]\n");
    let message = outcome(&text).expect_err("a second [target.demo_make] table must fail");
    assert!(
        message.contains("duplicate") && message.contains("demo_make"),
        "message does not name the duplicate id: {message}"
    );
}

#[test]
fn bed_01_manifest_rejects_missing_entry() {
    let no_entries = remove_between(
        SMALL,
        "[[target.demo_cmake.entry]]",
        "[[target.demo_cmake.config]]",
    );
    assert_fails_naming(&no_entries, "target.demo_cmake.entry");

    let extra = |count: usize| {
        let blocks: String = (0..count)
            .map(|i| {
                format!(
                    "[[target.demo_make.entry]]\nname = \"demo_extra{i}\"\nheader = \"inc/demo.h\"\nsource = \"src/demo.c\"\nform = \"direct\"\nsignature = \"int (const uint8_t *, size_t)\"\n\n"
                )
            })
            .collect();
        mutate(
            DEFAULT_MAKE_CONFIG,
            &format!("{blocks}{DEFAULT_MAKE_CONFIG}"),
        )
    };
    outcome(&extra(1)).expect("three entries are within the 1 to 3 bound");
    assert_fails_naming(&extra(2), "target.demo_make.entry");
}

#[test]
fn bed_01_manifest_rejects_copyleft_licence() {
    let cases: [(&str, &str); 6] = [
        (
            "spdx = \"GPL-2.0-only\"\nelected = \"GPL-2.0-only\"\n",
            "target.demo_make.licence.elected",
        ),
        (
            "spdx = \"LGPL-2.1-or-later\"\nelected = \"LGPL-2.1-or-later\"\n",
            "target.demo_make.licence.elected",
        ),
        (
            "spdx = \"MIT\"\nelected = \"BSD-3-Clause\"\n",
            "target.demo_make.licence.elected",
        ),
        (
            "spdx = \"GPL-2.0-or-later OR BSD-2-Clause\"\nelected = \"GPL-2.0-or-later\"\n",
            "target.demo_make.licence.elected",
        ),
        (
            "spdx = \"MIT AND BSD-3-Clause\"\nelected = \"MIT\"\n",
            "target.demo_make.licence.spdx",
        ),
        (
            "spdx = \"GPL-2.0-or-later WITH Classpath-exception-2.0\"\nelected = \"MIT\"\n",
            "target.demo_make.licence.spdx",
        ),
    ];
    for (licence, needle) in cases {
        assert_fails_naming(&mutate(MAKE_LICENCE, licence), needle);
    }
}

#[test]
fn bed_01_manifest_accepts_dual_licence_with_permissive_election() {
    let cases = [
        "spdx = \"GPL-2.0-or-later OR BSD-2-Clause\"\nelected = \"BSD-2-Clause\"\n",
        "spdx = \"Apache-2.0 OR GPL-2.0-or-later\"\nelected = \"Apache-2.0\"\n",
        "spdx = \"EPL-2.0 OR BSD-3-Clause\"\nelected = \"BSD-3-Clause\"\n",
    ];
    for licence in cases {
        outcome(&mutate(MAKE_LICENCE, licence))
            .unwrap_or_else(|e| panic!("{licence:?} was rejected: {e}"));
    }
    // The fixture's second target is already dual-licensed.
    let manifest = outcome(SMALL).expect("valid");
    assert_eq!(
        manifest.target["demo_cmake"].licence.elected,
        "BSD-2-Clause"
    );
}

#[test]
fn bed_01_wrapped_entry_requires_accepted_wrapped() {
    let without = mutate("accepted_wrapped = true\n", "");
    assert_fails_naming(&without, "accepted_wrapped");

    let on_direct = mutate(
        "form = \"direct\"\n",
        "form = \"direct\"\naccepted_wrapped = true\n",
    );
    assert_fails_naming(&on_direct, "accepted_wrapped");

    let on_range = mutate(
        "form = \"range\"\n",
        "form = \"range\"\naccepted_wrapped = true\n",
    );
    assert_fails_naming(&on_range, "accepted_wrapped");
}

#[test]
fn bed_01_manifest_rejects_malformed_pins() {
    let cases: [(String, &str); 9] = [
        (
            mutate(
                "0123456789abcdef0123456789abcdef01234567",
                "0123456789abcdef0123456789abcdef0123456",
            ),
            "target.demo_make.source.commit",
        ),
        (
            mutate(
                "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
                "00112233445566778899AABBCCDDEEFF00112233445566778899aabbccddeeff",
            ),
            "target.demo_make.source.tree_sha256",
        ),
        (
            mutate(
                "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100",
                "ffeeddccbbaa99887766554433221100ffeeddccbbaa9988776655443322110",
            ),
            "target.demo_cmake.source.sha256",
        ),
        (
            mutate(
                "89abcdef0123456789abcdef0123456789abcdef",
                "89abcdef0123456789abcdef0123456789abcdef0",
            ),
            "target.demo_cmake.source.commit",
        ),
        (
            mutate(
                "https://example.com/demo/make.git",
                "http://example.com/demo/make.git",
            ),
            "target.demo_make.source.url",
        ),
        (
            mutate(
                "https://example.com/demo/make.git",
                "https://example.com/demo make.git",
            ),
            "target.demo_make.source.url",
        ),
        (
            mutate("ref = \"v1.2.3\"", "ref = \"\""),
            "target.demo_make.source.ref",
        ),
        (
            mutate("strip_components = 1", "strip_components = 5"),
            "target.demo_cmake.source.strip_components",
        ),
        (
            mutate(
                "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100",
                "",
            ),
            "target.demo_cmake.source.sha256",
        ),
    ];
    for (text, needle) in cases {
        assert_fails_naming(&text, needle);
    }
}

#[test]
fn bed_01_manifest_rejects_unsafe_paths() {
    let cases: [(String, &str); 8] = [
        (
            mutate("file = \"LICENSE\"", "file = \"/etc/passwd\""),
            "target.demo_make.licence.file",
        ),
        (
            mutate("file = \"LICENSE\"", "file = \"../LICENSE\""),
            "target.demo_make.licence.file",
        ),
        (
            mutate("file = \"LICENSE\"", "file = \"LI\\u0000CENSE\""),
            "target.demo_make.licence.file",
        ),
        (
            mutate("file = \"LICENSE\"", "file = \"\""),
            "target.demo_make.licence.file",
        ),
        (
            mutate("header = \"inc/demo.h\"", "header = \"inc/../../demo.h\""),
            "header",
        ),
        (
            mutate("source = \"src/demo.c\"", "source = \"/abs/demo.c\""),
            "source",
        ),
        (
            mutate("dir = \".\"\ngoals", "dir = \"..\"\ngoals"),
            "target.demo_make.build_args.dir",
        ),
        (
            mutate("dir = \".\"\ncmake_target", "dir = \"/tmp\"\ncmake_target"),
            "target.demo_cmake.build_args.dir",
        ),
    ];
    for (text, needle) in cases {
        assert_fails_naming(&text, needle);
    }
}

#[test]
fn bed_01_manifest_rejects_unknown_placeholder() {
    let cases: [(String, &str); 4] = [
        (
            mutate("CC={cc} {cflags}", "CC={compiler} {cflags}"),
            "target.demo_make.build_args.make_vars",
        ),
        (
            mutate("-DDEMO_TESTS=OFF", "-DDEMO_TESTS={src"),
            "target.demo_cmake.build_args.cmake_options",
        ),
        (
            mutate("DEMO_NO_FLOAT", "DEMO_{baseline}"),
            "target.demo_make.config.no-float.defines",
        ),
        (
            mutate("-DDEMO_MAX_DEPTH=8", "-DDEMO_MAX_DEPTH={}"),
            "target.demo_cmake.config.depth-8.cmake_options",
        ),
    ];
    for (text, needle) in cases {
        assert_fails_naming(&text, needle);
    }

    for ok in ["{cc}", "{cflags}", "{src}", "{inputs}"] {
        let text = mutate("DEMO_NO_FLOAT", &format!("DEMO_PATH={ok}/x"));
        outcome(&text).unwrap_or_else(|e| panic!("placeholder {ok} was rejected: {e}"));
    }
}

#[test]
fn bed_01_validate_complete_requires_build_args() {
    let without = remove_between(
        SMALL,
        "[target.demo_cmake.build_args]",
        "[[target.demo_cmake.entry]]",
    );
    outcome(&without).expect("validate alone does not require a recipe");
    let message = outcome_complete(&without).expect_err("validate_complete must require a recipe");
    assert!(
        message.contains("target.demo_cmake.build_args: missing"),
        "message does not name the missing recipe: {message}"
    );
}

#[test]
fn bed_01_build_args_must_match_the_build_system() {
    let cases: [(String, &str); 4] = [
        (
            mutate("goals = [\"libdemo.a\"]\n", ""),
            "target.demo_make.build_args.goals",
        ),
        (
            mutate(
                "goals = [\"libdemo.a\"]\n",
                "goals = [\"libdemo.a\"]\ncmake_target = \"x\"\n",
            ),
            "target.demo_make.build_args.cmake_target",
        ),
        (
            mutate("cmake_target = \"demo_json\"\n", ""),
            "target.demo_cmake.build_args.cmake_target",
        ),
        (
            mutate(
                "cmake_target = \"demo_json\"\n",
                "cmake_target = \"demo_json\"\ngoals = [\"all\"]\n",
            ),
            "target.demo_cmake.build_args.goals",
        ),
    ];
    for (text, needle) in cases {
        assert_fails_naming(&text, needle);
    }
}

#[test]
fn bed_02_config_set_has_one_default_and_at_most_one_variant() {
    // A target with only the default configuration is complete (the isotp case).
    let default_only = remove_between(
        SMALL,
        "[[target.demo_make.config]]\nname = \"no-float\"",
        "[target.demo_cmake]",
    );
    outcome(&default_only).expect("a default-only target is valid");

    // Missing default.
    let no_default = mutate(DEFAULT_MAKE_CONFIG, "");
    assert_fails_naming(
        &no_default,
        "target.demo_make.config: exactly one configuration named default",
    );

    // Two variants.
    let two_variants = mutate(
        "[target.demo_cmake]\n",
        "[[target.demo_make.config]]\nname = \"other\"\ndefines = [\"X\"]\n\n[target.demo_cmake]\n",
    );
    assert_fails_naming(
        &two_variants,
        "target.demo_make.config: at most one variant",
    );

    // Duplicate names.
    let duplicate = mutate(
        "[target.demo_cmake]\n",
        "[[target.demo_make.config]]\nname = \"no-float\"\ndefines = [\"X\"]\n\n[target.demo_cmake]\n",
    );
    assert_fails_naming(
        &duplicate,
        "target.demo_make.config: name \"no-float\" is used twice",
    );

    // A default that sets something.
    let loaded_default = mutate(
        DEFAULT_MAKE_CONFIG,
        &format!("{DEFAULT_MAKE_CONFIG}defines = [\"A\"]\n"),
    );
    assert_fails_naming(
        &loaded_default,
        "target.demo_make.config.default: must set nothing",
    );

    // A badly spelled name.
    let bad_name = mutate("name = \"no-float\"", "name = \"No_Float\"");
    assert_fails_naming(&bad_name, "target.demo_make.config.No_Float.name");
}

#[test]
fn bed_02_config_settings_must_match_the_build_system() {
    let cmake_on_make = mutate(
        MAKE_VARIANT,
        "name = \"no-float\"\ncmake_options = [\"-DX=1\"]\n",
    );
    assert_fails_naming(
        &cmake_on_make,
        "target.demo_make.config.no-float.cmake_options",
    );

    let target_on_make = mutate(
        MAKE_VARIANT,
        "name = \"no-float\"\ncmake_target = \"other\"\n",
    );
    assert_fails_naming(
        &target_on_make,
        "target.demo_make.config.no-float.cmake_target",
    );

    let make_on_cmake = mutate(
        "name = \"depth-8\"\ncmake_options = [\"-DDEMO_MAX_DEPTH=8\"]\n",
        "name = \"depth-8\"\nmake_vars = [\"X=1\"]\n",
    );
    assert_fails_naming(&make_on_cmake, "target.demo_cmake.config.depth-8.make_vars");

    let goals_on_cmake = mutate(
        "name = \"depth-8\"\ncmake_options = [\"-DDEMO_MAX_DEPTH=8\"]\n",
        "name = \"depth-8\"\ngoals = [\"all\"]\n",
    );
    assert_fails_naming(&goals_on_cmake, "target.demo_cmake.config.depth-8.goals");
}

#[test]
fn bed_02_empty_variant_is_rejected() {
    let empty = mutate(MAKE_VARIANT, "name = \"no-float\"\n");
    assert_fails_naming(&empty, "target.demo_make.config.no-float: sets nothing");
}

#[test]
fn bed_02_variant_equal_to_default_is_rejected() {
    // The variant repeats the recipe's own make variable, so the build is the default build.
    let same_make = mutate(
        MAKE_VARIANT,
        "name = \"no-float\"\nmake_vars = [\"CC={cc} {cflags}\"]\n",
    );
    assert_fails_naming(
        &same_make,
        "target.demo_make.config.no-float: every setting already appears",
    );

    let same_cmake = mutate(
        "name = \"depth-8\"\ncmake_options = [\"-DDEMO_MAX_DEPTH=8\"]\n",
        "name = \"depth-8\"\ncmake_options = [\"-DDEMO_TESTS=OFF\"]\n",
    );
    assert_fails_naming(
        &same_cmake,
        "target.demo_cmake.config.depth-8: every setting already appears",
    );

    // The same variable plus a define does change the build.
    let plus_define = mutate(
        MAKE_VARIANT,
        "name = \"no-float\"\nmake_vars = [\"CC={cc} {cflags}\"]\ndefines = [\"DEMO_NO_FLOAT\"]\n",
    );
    outcome(&plus_define).expect("a define on top of a repeated variable is a real variant");
}

#[test]
fn bed_02_config_order_does_not_change_canonical_form() {
    let reference = outcome(SMALL)
        .expect("valid")
        .canonical_json()
        .expect("json");

    // Configurations: move the default after the variant, in the text.
    let variant_first = mutate(DEFAULT_MAKE_CONFIG, "").replace(
        "[target.demo_cmake]\n",
        &format!("{DEFAULT_MAKE_CONFIG}\n[target.demo_cmake]\n"),
    );
    let shuffled = outcome(&variant_first)
        .expect("valid")
        .canonical_json()
        .expect("json");
    assert_eq!(
        reference, shuffled,
        "config order leaked into the canonical form"
    );

    // Entries: move the first entry after the second, in the text.
    let first_entry = first_entry_text();
    let moved = SMALL.replace(&first_entry, "").replace(
        DEFAULT_MAKE_CONFIG,
        &format!("{first_entry}{DEFAULT_MAKE_CONFIG}"),
    );
    assert_ne!(moved, SMALL, "the entry order was not changed");
    let entries_swapped = outcome(&moved)
        .expect("valid")
        .canonical_json()
        .expect("json");
    assert_eq!(
        reference, entries_swapped,
        "entry order leaked into the canonical form"
    );

    // The canonical text ends with exactly one newline and lists default first.
    assert!(reference.ends_with("}\n") && !reference.ends_with("\n\n"));
    let default_at = reference.find("\"default\"").expect("default present");
    let variant_at = reference.find("\"no-float\"").expect("variant present");
    assert!(default_at < variant_at);
}

/// The `demo_decode` entry table of the fixture, verbatim.
fn first_entry_text() -> String {
    let start = SMALL
        .find("[[target.demo_make.entry]]\nname = \"demo_decode\"")
        .expect("first entry present");
    let end = SMALL
        .find("[[target.demo_make.entry]]\nname = \"demo_next\"")
        .expect("second entry present");
    SMALL[start..end].to_owned()
}

#[test]
fn bed_02_baseline_must_be_the_cortex_m4_hard_float_set() {
    let flags = "cflags = [\"-mcpu=cortex-m4\", \"-mthumb\", \"-mfloat-abi=hard\", \"-mfpu=fpv4-sp-d16\", \"-Os\"]";
    let cases = [
        // Soft-float ABI.
        "cflags = [\"-mcpu=cortex-m4\", \"-mthumb\", \"-mfloat-abi=softfp\", \"-mfpu=fpv4-sp-d16\", \"-Os\"]",
        // No optimisation flag.
        "cflags = [\"-mcpu=cortex-m4\", \"-mthumb\", \"-mfloat-abi=hard\", \"-mfpu=fpv4-sp-d16\"]",
        // Reordered.
        "cflags = [\"-mthumb\", \"-mcpu=cortex-m4\", \"-mfloat-abi=hard\", \"-mfpu=fpv4-sp-d16\", \"-Os\"]",
        // An extra flag.
        "cflags = [\"-mcpu=cortex-m4\", \"-mthumb\", \"-mfloat-abi=hard\", \"-mfpu=fpv4-sp-d16\", \"-Os\", \"-g\"]",
        // A different core.
        "cflags = [\"-mcpu=cortex-m3\", \"-mthumb\", \"-mfloat-abi=hard\", \"-mfpu=fpv4-sp-d16\", \"-Os\"]",
    ];
    for replacement in cases {
        assert_fails_naming(&mutate(flags, replacement), "baseline.cflags");
    }
}

#[test]
fn bed_02_decision_record_is_validated() {
    let cases: [(String, &str); 5] = [
        (
            mutate("first_rtos = \"deferred\"", "first_rtos = \"threadx\""),
            "decisions.first_rtos",
        ),
        (
            mutate(
                "first_rtos_reason = \"parsers are RTOS-independent; revisit after the design-partner interviews\"",
                "first_rtos_reason = \"\"",
            ),
            "decisions.first_rtos_reason",
        ),
        (
            mutate(
                "first_rtos_candidates = [\"freertos\", \"zephyr\"]",
                "first_rtos_candidates = []",
            ),
            "decisions.first_rtos_candidates",
        ),
        (
            mutate(
                "project_decision = \"D-18\"",
                "project_decision = \"decision 18\"",
            ),
            "decisions.project_decision",
        ),
        (
            mutate("schema_version = 1", "schema_version = 2"),
            "unsupported manifest schema_version 2",
        ),
    ];
    for (text, needle) in cases {
        assert_fails_naming(&text, needle);
    }
}

#[test]
fn bed_01_image_digest_must_be_sha256_and_64_lowercase_hex() {
    let good = "sha256:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9";
    assert!(SMALL.contains(good), "the fixture names the image digest");
    for bad in [
        "sha256:e83006c0",
        "e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9",
        "sha256:E83006C0A7E37F788AFCBD1C95554E8C4DFC61C3B68DA3FF964CAF47FB3851B9",
        "sha512:e83006c0a7e37f788afcbd1c95554e8c4dfc61c3b68da3ff964caf47fb3851b9",
        "",
    ] {
        assert_fails_naming(&mutate(good, bad), "image.digest");
    }
    // A manifest with no [image] table is a parse error, not a default.
    let without = remove_between(SMALL, "[image]", "[baseline]");
    assert!(outcome(&without).unwrap_err().contains("image"));
}
