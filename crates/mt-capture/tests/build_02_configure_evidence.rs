#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-01 / BUILD-02 configure-time evidence of a CMake capture (gap G-02-2,
//! review WR-10).
//!
//! Every test here is pure: it reads no tool, starts no process and needs no
//! network, so the tool-free `ci` workflow runs it. The capture-level proofs of
//! the same rules are in `build_01_capture_drivers.rs`.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use mt_capture::capture::{
    CONFIGURE_HASH_EXCLUSIONS, NORMALISED_CONFIGURE_FILES, excluded_from_configure_hashes,
    is_cmake_product, normalise_make_driver,
};

/// A small make driver in the shape CMake's Unix Makefiles generator writes.
const DRIVER: &str = "\
# Default target executed when no arguments are given to make.
default_target: all
.PHONY : default_target

all: CMakeFiles/pet.dir/all
all: CMakeFiles/app.dir/all
.PHONY : all

clean: CMakeFiles/pet.dir/clean
clean: CMakeFiles/app.dir/clean
.PHONY : clean

CMakeFiles/app.dir/all: CMakeFiles/pet1_yaml.dir/all
CMakeFiles/app.dir/all: CMakeFiles/pet.dir/all
	$(MAKE) -f CMakeFiles/app.dir/build.make CMakeFiles/app.dir/depend
	$(MAKE) -f CMakeFiles/app.dir/build.make CMakeFiles/app.dir/build
.PHONY : CMakeFiles/app.dir/all
";

fn normalised(text: &str) -> String {
    String::from_utf8(normalise_make_driver(text.as_bytes())).unwrap()
}

fn swap_lines(text: &str, first: &str, second: &str) -> String {
    let swapped = text
        .replace(first, "@@FIRST@@")
        .replace(second, first)
        .replace("@@FIRST@@", second);
    assert_ne!(swapped, text, "the swap must change the text");
    swapped
}

#[test]
fn build_02_make_driver_normalisation_sorts_runs_of_one_target() {
    // The measured nondeterminism: two equal-target prerequisite lines swap.
    let swapped = swap_lines(
        DRIVER,
        "CMakeFiles/app.dir/all: CMakeFiles/pet1_yaml.dir/all\n",
        "CMakeFiles/app.dir/all: CMakeFiles/pet.dir/all\n",
    );
    assert_ne!(swapped, DRIVER);
    assert_eq!(normalised(&swapped), normalised(DRIVER));

    // The sort is bytewise within the run and every other line stays in place.
    let expected_run = "CMakeFiles/app.dir/all: CMakeFiles/pet.dir/all\n\
                        CMakeFiles/app.dir/all: CMakeFiles/pet1_yaml.dir/all\n";
    assert!(normalised(DRIVER).contains(expected_run));
    assert!(
        normalised(DRIVER).contains("all: CMakeFiles/app.dir/all\nall: CMakeFiles/pet.dir/all\n")
    );

    // One changed prerequisite is a different hash input.
    let changed = DRIVER.replace(
        "CMakeFiles/app.dir/all: CMakeFiles/pet.dir/all\n",
        "CMakeFiles/app.dir/all: CMakeFiles/other.dir/all\n",
    );
    assert_ne!(normalised(&changed), normalised(DRIVER));

    // A prerequisite line moved into another target's run is a change: the
    // `app.dir/all` line now follows the `all:` lines, so it is no longer one
    // run with the other `app.dir/all` line.
    let moved = DRIVER
        .replace("CMakeFiles/app.dir/all: CMakeFiles/pet.dir/all\n", "")
        .replace(
            "all: CMakeFiles/app.dir/all\n",
            "all: CMakeFiles/app.dir/all\nCMakeFiles/app.dir/all: CMakeFiles/pet.dir/all\n",
        );
    assert_ne!(normalised(&moved), normalised(DRIVER));

    // Two different targets' runs reordered are a change.
    let reordered = swap_lines(
        DRIVER,
        "all: CMakeFiles/pet.dir/all\nall: CMakeFiles/app.dir/all\n.PHONY : all\n",
        "clean: CMakeFiles/pet.dir/clean\nclean: CMakeFiles/app.dir/clean\n.PHONY : clean\n",
    );
    assert_ne!(normalised(&reordered), normalised(DRIVER));
}

#[test]
fn build_02_make_driver_normalisation_keeps_other_lines_in_place() {
    // `.PHONY : x` (a space before the colon), comments and tab-indented recipe
    // lines are not prerequisite lines: they split runs and never move.
    let text = "a: z\n.PHONY : a\na: b\n# note: x\na: y\n\ta: not a rule\na: c\n";
    assert_eq!(normalised(text), text);

    // Within one run the sort is bytewise, and the lines around it stay put.
    let text = "# head\nt: z\nt: b\nt: m\n.PHONY : t\n\trecipe: a\nu: q\nu: p\n";
    assert_eq!(
        normalised(text),
        "# head\nt: b\nt: m\nt: z\n.PHONY : t\n\trecipe: a\nu: p\nu: q\n"
    );

    // Different targets next to each other are different runs.
    let text = "b: 2\na: 1\n";
    assert_eq!(normalised(text), text);
}

#[test]
fn build_02_make_driver_normalisation_keeps_the_trailing_newline() {
    assert_eq!(normalised("a: z\na: b\n"), "a: b\na: z\n");
    assert_eq!(normalised("a: z\na: b"), "a: b\na: z");
    assert_eq!(normalised(""), "");
    assert_eq!(normalised("\n"), "\n");
    // Non-UTF-8 bytes pass through untouched.
    assert_eq!(normalise_make_driver(b"\xff\xfe\n"), b"\xff\xfe\n");
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/makefile2")
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn build_02_real_makefile2_variants_normalise_equal() {
    // Two Makefile2 files CMake wrote for the same zcbor sample (20 configures
    // gave exactly these two variants, 12 and 8 times), with the build and source
    // prefixes rewritten to the run layout. They differ only in the order of the
    // two `app.dir/all` prerequisite lines.
    let a = fixture("zcbor-pet-a.txt");
    let b = fixture("zcbor-pet-b.txt");

    assert_ne!(a, b, "the variants must differ as bytes");
    assert_eq!(normalise_make_driver(&a), normalise_make_driver(&b));
}

/// The paths CMake itself writes and the configure-time hashes leave out,
/// whatever the declared objects and the pre-build listing hold.
const PRODUCTS: [&str; 7] = [
    "CMakeFiles/CMakeConfigureLog.yaml",
    "CMakeFiles/CMakeOutput.log",
    "CMakeFiles/CMakeError.log",
    "CMakeFiles/3.25.1/CMakeDetermineCompilerABI_C.bin",
    "CMakeFiles/3.28.3/CMakeDetermineCompilerABI_CXX.bin",
    "CMakeFiles/3.25.1/CompilerIdC/CMakeCCompilerId.o",
    "CMakeFiles/3.25.1/CompilerIdCXX/CMakeCXXCompilerId.o",
];

#[test]
fn build_01_cmake_products_are_exactly_the_documented_paths() {
    for product in PRODUCTS {
        assert!(is_cmake_product(product), "{product} is a CMake product");
    }
    for other in [
        // The make driver is hashed, normalised.
        "CMakeFiles/Makefile2",
        "sub/CMakeFiles/Makefile2",
        // Suffixes alone decide nothing any more.
        "x.o",
        "gen/table.a",
        "data.so",
        "CMakeFiles/tj.dir/t.c.obj",
        "libtj.a",
        // The logs and probe outputs only at their documented depth.
        "sub/CMakeFiles/CMakeOutput.log",
        "CMakeFiles/x/y/CMakeDetermineCompilerABI_C.bin",
        "CMakeFiles/abc/CMakeDetermineCompilerABI_C.bin",
        "CMakeFiles/3.25.1/CMakeDetermineCompilerABI_ASM.bin",
        "CMakeFiles/3.25.1/CMakeDetermineCompilerABI_C.bin.bak",
        "CMakeFiles/3.25.1/CompilerIdC/other.o",
        "CMakeFiles/3.25.1/CompilerIdC/CMakeCXXCompilerId.o",
        "CMakeFiles/3.25.1/CompilerIdCXX/CMakeCCompilerId.o",
        "CMakeFiles/CompilerIdC/CMakeCCompilerId.o",
        "CMakeFiles//CompilerIdC/CMakeCCompilerId.o",
        "CMakeFiles/3.25.1/CompilerIdC/sub/CMakeCCompilerId.o",
        "tables/CMakeFiles/3.25.1/CMakeDetermineCompilerABI_C.bin",
    ] {
        assert!(!is_cmake_product(other), "{other} is not a CMake product");
    }
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn build_01_configure_hash_skip_decision_matches_the_documented_set() {
    let declared = set(&[
        "build/CMakeFiles/tj.dir/t.c.obj",
        "build/CMakeFiles/tj.dir/t.c.o",
    ]);
    let nothing = BTreeSet::new();

    // Objects named in the capture report: skipped by declaration.
    for object in ["CMakeFiles/tj.dir/t.c.obj", "CMakeFiles/tj.dir/t.c.o"] {
        assert!(excluded_from_configure_hashes(object, &declared, &nothing));
    }
    // Archives and shared libraries the build step created (absent from the
    // listing taken after configure): skipped by provenance.
    for created in ["libtj.a", "libtj.so"] {
        assert!(excluded_from_configure_hashes(created, &nothing, &nothing));
    }
    // CMake's own outputs: skipped whatever the sets hold.
    let listed: BTreeSet<String> = PRODUCTS
        .iter()
        .map(|path| format!("build/{path}"))
        .collect();
    for product in PRODUCTS {
        assert!(excluded_from_configure_hashes(product, &nothing, &nothing));
        assert!(excluded_from_configure_hashes(product, &declared, &listed));
    }

    // Configure-time look-alikes, present before the build: hashed.
    let before = set(&["build/libtj.a", "build/libtj.so", "build/gen/table.a"]);
    for configure_time in ["libtj.a", "libtj.so", "gen/table.a"] {
        assert!(
            !excluded_from_configure_hashes(configure_time, &declared, &before),
            "{configure_time} existed before the build"
        );
    }
    // No suffix rule: an undeclared object is hashed.
    assert!(!excluded_from_configure_hashes(
        "CMakeFiles/tj.dir/t.c.obj",
        &nothing,
        &nothing
    ));
    // The declared set holds run-root keys, so a key without `build/` is no match.
    assert!(!excluded_from_configure_hashes(
        "CMakeFiles/tj.dir/t.c.obj",
        &set(&["CMakeFiles/tj.dir/t.c.obj"]),
        &nothing
    ));
    // Exactly `.a` and `.so`.
    for other in ["libtj.a.bak", "data.so.1"] {
        assert!(!excluded_from_configure_hashes(other, &nothing, &nothing));
    }
    // Only archives and shared libraries get the provenance skip.
    assert!(!excluded_from_configure_hashes(
        "bin/app.elf",
        &nothing,
        &nothing
    ));
    // The make driver is hashed normalised, never skipped.
    assert!(!excluded_from_configure_hashes(
        "CMakeFiles/Makefile2",
        &nothing,
        &nothing
    ));
}

#[test]
fn build_01_configure_hash_exclusions_are_published() {
    assert_eq!(
        CONFIGURE_HASH_EXCLUSIONS,
        [
            "objects named in the capture report",
            "CMakeFiles/CMakeConfigureLog.yaml, CMakeFiles/CMakeOutput.log and CMakeFiles/CMakeError.log",
            "CMakeFiles/<version>/CMakeDetermineCompilerABI_<C or CXX>.bin",
            "CMakeFiles/<version>/CompilerId<C or CXX>/CMake<C or CXX>CompilerId.o",
            "static archives and shared libraries (.a, .so) created by the cmake build step",
        ]
    );
    assert_eq!(
        NORMALISED_CONFIGURE_FILES,
        ["CMakeFiles/Makefile2, hashed with each run of prerequisite lines of one target sorted"]
    );
}
