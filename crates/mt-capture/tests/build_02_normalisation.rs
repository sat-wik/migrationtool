#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BUILD-01 architecture gate and BUILD-02 section comparison with the fixed
//! normalisation (CONTEXT D-09).
//!
//! The readelf-text tests run anywhere. The object tests build host objects with
//! `/usr/bin/cc` and edit them with `/usr/bin/objcopy`, both through the runner,
//! so they run on Linux only. Real Arm objects from CI are published by plan
//! 02-06's workflow and committed as fixtures in plan 02-10.

use mt_capture::elf::{ArmAttributes, normalisable, parse_readelf_a};

/// `readelf -A` of a Cortex-M4 hard-float object (the lines of RESEARCH
/// "Effective-flags gate", in readelf's own layout).
const CORTEX_M4_HARD_FLOAT: &str = "\
Attribute Section: aeabi
File Attributes
  Tag_CPU_name: \"Cortex-M4\"
  Tag_CPU_arch: v7E-M
  Tag_CPU_arch_profile: Microcontroller
  Tag_THUMB_ISA_use: Thumb-2
  Tag_FP_arch: VFPv4-D16
  Tag_ABI_PCS_wchar_t: 4
  Tag_ABI_FP_denormal: Needed
  Tag_ABI_FP_exceptions: Needed
  Tag_ABI_FP_number_model: IEEE 754
  Tag_ABI_align_needed: 8-byte
  Tag_ABI_enum_size: small
  Tag_ABI_VFP_args: VFP registers
  Tag_ABI_optimization_goals: Aggressive Size
";

fn without_line(text: &str, tag: &str) -> String {
    text.lines()
        .filter(|line| !line.contains(tag))
        .map(|line| format!("{line}\n"))
        .collect()
}

fn attributes(text: &str) -> ArmAttributes {
    parse_readelf_a(text)
}

#[test]
fn build_01_architecture_gate_parses_readelf_attributes() {
    let good = attributes(CORTEX_M4_HARD_FLOAT);
    assert_eq!(good.cpu_arch.as_deref(), Some("v7E-M"));
    assert_eq!(good.thumb_isa_use.as_deref(), Some("Thumb-2"));
    assert_eq!(good.fp_arch.as_deref(), Some("VFPv4-D16"));
    assert_eq!(good.abi_vfp_args.as_deref(), Some("VFP registers"));
    assert_eq!(good.optimization_goals.as_deref(), Some("Aggressive Size"));
    assert!(good.is_cortex_m4_hard_float());

    // Another core.
    let wrong_core = CORTEX_M4_HARD_FLOAT.replace("Tag_CPU_arch: v7E-M", "Tag_CPU_arch: v6S-M");
    assert!(!attributes(&wrong_core).is_cortex_m4_hard_float());
    // The default FPU tag is absent when the object was built soft-float.
    let no_fpu = without_line(CORTEX_M4_HARD_FLOAT, "Tag_FP_arch");
    assert_eq!(attributes(&no_fpu).fp_arch, None);
    assert!(!attributes(&no_fpu).is_cortex_m4_hard_float());
    let no_vfp_args = without_line(CORTEX_M4_HARD_FLOAT, "Tag_ABI_VFP_args");
    assert!(!attributes(&no_vfp_args).is_cortex_m4_hard_float());
    // The wrong FPU or ABI also fails.
    let single = CORTEX_M4_HARD_FLOAT.replace("VFPv4-D16", "VFPv4");
    assert!(!attributes(&single).is_cortex_m4_hard_float());
    let soft_args = CORTEX_M4_HARD_FLOAT.replace("VFP registers", "AAPCS");
    assert!(!attributes(&soft_args).is_cortex_m4_hard_float());
    // The optimisation goal is recorded, never enforced (http-parser appends -O3).
    let no_goal = without_line(CORTEX_M4_HARD_FLOAT, "Tag_ABI_optimization_goals");
    let relaxed = attributes(&no_goal);
    assert_eq!(relaxed.optimization_goals, None);
    assert!(relaxed.is_cortex_m4_hard_float());
    // Nothing to read is nothing passed.
    assert!(!attributes("").is_cortex_m4_hard_float());
    assert!(!attributes("readelf: Error: Not an ELF file\n").is_cortex_m4_hard_float());
}

#[test]
fn build_02_normalisable_accepts_only_comment_and_debug_names() {
    let names = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert!(normalisable(&names(&[".comment"])));
    assert!(normalisable(&names(&[
        ".comment",
        ".debug_info",
        ".debug_line_str",
        ".rel.debug_info",
        ".rela.debug_line"
    ])));
    // An empty set is not a difference to forgive.
    assert!(!normalisable(&[]));
    for forbidden in [
        ".text",
        ".data",
        ".rodata",
        ".symtab",
        ".strtab",
        ".ARM.attributes",
        ".debugger",
        ".rel.text",
    ] {
        assert!(!normalisable(&names(&[forbidden])), "{forbidden}");
        assert!(
            !normalisable(&names(&[".comment", forbidden])),
            "{forbidden}"
        );
    }
}

#[cfg(target_os = "linux")]
mod host_objects {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};

    use mt_capture::cc::{CcEntry, CcPolicy};
    use mt_capture::elf::{
        ElfError, check_elf_header, code_digest, differing_sections, parse_readelf_a, sections,
    };
    use mt_capture::replay::{Normaliser, ReplayPlan, replay};
    use mt_capture::report::{ObjectStatus, ReplayReport, Verdict};
    use mt_capture::treehash::sha256_hex;
    use mt_toolchain::runner::{RunRequest, RunnerConfig, run};
    use tempfile::TempDir;

    const CC: &str = "/usr/bin/cc";
    const OBJCOPY: &str = "/usr/bin/objcopy";
    const READELF: &str = "/usr/bin/readelf";
    const KEY: &str = "src/out/x.o";
    const SOURCE: &str = "#ifndef V\n#define V 0\n#endif\nint v(void) { return V; }\nstatic int data = 5;\nint d(void) { return data; }\n";

    fn config() -> RunnerConfig {
        RunnerConfig::new("/usr/bin:/bin", "1791158400")
    }

    fn request(argv: &[&str], cwd: &Path) -> RunRequest {
        RunRequest {
            tool_name: "host".to_owned(),
            argv: argv.iter().map(OsString::from).collect(),
            cwd: cwd.to_path_buf(),
            extra_env: BTreeMap::new(),
            tool_version: None,
        }
    }

    fn run_ok(argv: &[&str], cwd: &Path) -> Vec<u8> {
        let output = run(&config(), &request(argv, cwd)).unwrap();
        assert_eq!(output.record.exit.code, Some(0), "{argv:?}");
        output.stdout
    }

    /// Build `x.c` in `dir/src` with `cc <args> -c -o out/x.o x.c`; return the object.
    fn build_object(dir: &Path, args: &[&str]) -> Vec<u8> {
        let src = dir.join("src");
        fs::create_dir_all(src.join("out")).unwrap();
        fs::write(src.join("x.c"), SOURCE).unwrap();
        let mut argv = vec![CC];
        argv.extend_from_slice(args);
        argv.extend_from_slice(&["-c", "-o", "out/x.o", "x.c"]);
        run_ok(&argv, &src);
        fs::read(src.join("out/x.o")).unwrap()
    }

    /// The same object with a different `.comment` section.
    fn with_other_comment(scratch: &Path, object: &[u8]) -> Vec<u8> {
        fs::create_dir_all(scratch).unwrap();
        fs::write(scratch.join("in.o"), object).unwrap();
        fs::write(scratch.join("comment.bin"), b"another compiler 1.0\0").unwrap();
        run_ok(
            &[
                OBJCOPY,
                "--update-section",
                ".comment=comment.bin",
                "in.o",
                "out.o",
            ],
            scratch,
        );
        fs::read(scratch.join("out.o")).unwrap()
    }

    struct Setup {
        tmp: TempDir,
        run_root: PathBuf,
        originals: PathBuf,
    }

    impl Setup {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let run_root = tmp.path().join("run");
            let originals = tmp.path().join("originals");
            fs::create_dir_all(&run_root).unwrap();
            Self {
                tmp,
                run_root,
                originals,
            }
        }

        fn pristine(&self) -> PathBuf {
            let pristine = self.tmp.path().join("pristine");
            fs::create_dir_all(&pristine).unwrap();
            fs::write(pristine.join("x.c"), SOURCE).unwrap();
            pristine
        }

        fn scratch(&self) -> PathBuf {
            self.tmp.path().join("scratch")
        }

        /// A plan that replays `cc <args> -c -o out/x.o x.c` against `original`.
        fn plan(&self, original: &[u8], args: &[&str], normaliser: bool) -> ReplayPlan {
            let saved = self.originals.join(KEY);
            fs::create_dir_all(saved.parent().unwrap()).unwrap();
            fs::write(&saved, original).unwrap();
            let directory = self.run_root.join("src");
            let mut arguments = vec![CC.to_owned()];
            arguments.extend(args.iter().map(|s| (*s).to_owned()));
            arguments.extend(["-c", "-o", "out/x.o", "x.c"].map(str::to_owned));
            ReplayPlan {
                target: "host".to_owned(),
                config: "default".to_owned(),
                run_root: self.run_root.clone(),
                pristine_src: self.pristine(),
                src_dir_name: "src".to_owned(),
                keep_dirs: Vec::new(),
                entries: vec![CcEntry {
                    directory: directory.to_str().unwrap().to_owned(),
                    file: "x.c".to_owned(),
                    arguments,
                    output: directory.join("out/x.o").to_str().unwrap().to_owned(),
                }],
                original_objects: BTreeMap::from([(KEY.to_owned(), sha256_hex(original))]),
                original_objects_dir: self.originals.clone(),
                policy: CcPolicy {
                    allowed_compilers: vec![CC.to_owned()],
                    run_root: self.run_root.clone(),
                },
                compiler_tool_name: "cc".to_owned(),
                normaliser: normaliser.then(|| Normaliser {
                    objcopy: PathBuf::from(OBJCOPY),
                    scratch: self.scratch(),
                }),
            }
        }
    }

    fn step_names(report: &ReplayReport) -> Vec<&str> {
        report.steps.iter().map(|step| step.name.as_str()).collect()
    }

    #[test]
    fn build_01_captured_objects_are_cortex_m4_hard_float() {
        let setup = Setup::new();
        let object = build_object(setup.tmp.path(), &[]);
        // A host cc object is ELF64 x86-64: the machine check refuses it.
        assert!(matches!(
            check_elf_header(&object),
            Err(ElfError::NotElf32 { class: 2 })
        ));
        assert!(matches!(check_elf_header(b"MZ"), Err(ElfError::NotElf)));
        assert!(matches!(check_elf_header(&[]), Err(ElfError::NotElf)));
        // Its readelf -A carries no Arm attributes, so the gate fails closed.
        let src = setup.tmp.path().join("src");
        let text = run_ok(&[READELF, "-A", "out/x.o"], &src);
        let attributes = parse_readelf_a(&String::from_utf8(text).unwrap());
        assert!(!attributes.is_cortex_m4_hard_float());
        assert_eq!(attributes.cpu_arch, None);
        // A hand-built ARM header passes the machine check.
        let mut arm = vec![0u8; 52];
        arm[..4].copy_from_slice(b"\x7fELF");
        arm[4] = 1; // ELFCLASS32
        arm[5] = 1; // ELFDATA2LSB
        arm[16] = 1; // ET_REL
        arm[18] = 40; // EM_ARM
        check_elf_header(&arm).unwrap();
        arm[18] = 3; // EM_386
        assert!(matches!(
            check_elf_header(&arm),
            Err(ElfError::NotArm { machine: 3 })
        ));
        arm[18] = 40;
        arm[16] = 2; // ET_EXEC
        assert!(matches!(
            check_elf_header(&arm),
            Err(ElfError::NotRelocatable { e_type: 2 })
        ));
    }

    #[test]
    fn build_02_normalisation_only_forgives_comment_and_debug_sections() {
        // .comment alone.
        let setup = Setup::new();
        let fresh = build_object(&setup.tmp.path().join("fresh"), &[]);
        let original = with_other_comment(&setup.tmp.path().join("edit"), &fresh);
        assert_eq!(
            differing_sections(&original, &fresh).unwrap(),
            [".comment".to_owned()]
        );
        let report = replay(&config(), &setup.plan(&original, &[], true)).unwrap();
        assert_eq!(
            report.verdict,
            Verdict::IdenticalAfterNormalisation,
            "{report:?}"
        );
        let object = &report.objects[KEY];
        assert_eq!(object.status, ObjectStatus::IdenticalAfterNormalisation);
        assert_eq!(object.differing_sections, [".comment"]);
        assert!(object.normalised_original.is_some());
        assert_eq!(object.normalised_original, object.normalised_replay);
        assert_ne!(object.original, object.replay);
        assert!(report.normalisation_applied);
        assert_eq!(
            step_names(&report),
            [
                "compile x.c",
                "normalise original src/out/x.o",
                "normalise replay src/out/x.o"
            ]
        );
        assert!(report.to_json().unwrap().contains("normalisation_rule"));

        // Debug sections alone: the same source compiled with -g in two directories.
        let setup = Setup::new();
        let original = build_object(&setup.tmp.path().join("elsewhere"), &["-g"]);
        let report = replay(&config(), &setup.plan(&original, &["-g"], true)).unwrap();
        let object = &report.objects[KEY];
        assert!(!object.differing_sections.is_empty(), "{report:?}");
        assert!(
            object
                .differing_sections
                .iter()
                .all(|name| name.starts_with(".debug_")
                    || name.starts_with(".rel.debug_")
                    || name.starts_with(".rela.debug_")),
            "{:?}",
            object.differing_sections
        );
        assert_eq!(
            report.verdict,
            Verdict::IdenticalAfterNormalisation,
            "{report:?}"
        );
        assert!(report.normalisation_applied);
    }

    #[test]
    fn build_02_normalisation_rejects_text_section_difference() {
        let setup = Setup::new();
        let original = build_object(&setup.tmp.path().join("one"), &["-DV=1"]);
        let report = replay(&config(), &setup.plan(&original, &["-DV=2"], true)).unwrap();

        assert_eq!(report.verdict, Verdict::Differs, "{report:?}");
        let object = &report.objects[KEY];
        assert_eq!(object.status, ObjectStatus::Differs);
        assert!(object.differing_sections.contains(&".text".to_owned()));
        assert_eq!(object.normalised_original, None);
        assert_eq!(object.normalised_replay, None);
        assert!(!report.normalisation_applied);
        // No objcopy ran.
        assert_eq!(step_names(&report), ["compile x.c"]);
    }

    #[test]
    fn build_02_equal_objects_are_identical_without_normalisation() {
        let setup = Setup::new();
        let original = build_object(&setup.tmp.path().join("one"), &[]);
        let report = replay(&config(), &setup.plan(&original, &[], true)).unwrap();

        assert_eq!(report.verdict, Verdict::Identical, "{report:?}");
        assert_eq!(report.objects[KEY].status, ObjectStatus::Identical);
        assert_eq!(report.objects[KEY].normalised_original, None);
        assert!(!report.normalisation_applied);
        assert_eq!(step_names(&report), ["compile x.c"]);
        // The rule is written into the report whether or not it was applied.
        assert!(report.normalisation_rule.contains("--strip-debug"));
        assert!(report.to_json().unwrap().contains("normalisation_rule"));
    }

    #[test]
    fn build_02_unparseable_object_is_reported_as_differs() {
        let setup = Setup::new();
        let report = replay(&config(), &setup.plan(b"this is not an object", &[], true)).unwrap();

        assert_eq!(report.verdict, Verdict::Differs, "{report:?}");
        let object = &report.objects[KEY];
        assert_eq!(object.status, ObjectStatus::Differs);
        let detail = object.detail.as_deref().unwrap();
        assert!(detail.contains("cannot read the object"), "{detail}");
        assert_eq!(object.normalised_original, None);
        assert!(!report.normalisation_applied);
        assert_eq!(step_names(&report), ["compile x.c"]);
        // And the reader itself returns a typed error.
        assert!(sections(b"this is not an object").is_err());
        assert!(sections(&[]).is_err());
    }

    #[test]
    fn build_02_code_digest_ignores_non_code_sections() {
        let setup = Setup::new();
        let base = build_object(&setup.tmp.path().join("base"), &[]);
        let other_comment = with_other_comment(&setup.tmp.path().join("edit"), &base);
        let other_value = build_object(&setup.tmp.path().join("value"), &["-DV=2"]);

        assert_ne!(base, other_comment);
        let digest = code_digest(&base).unwrap();
        assert_eq!(digest.len(), 64);
        assert_eq!(digest, code_digest(&other_comment).unwrap());
        assert_ne!(digest, code_digest(&other_value).unwrap());
        assert!(code_digest(b"junk").is_err());
        // sections() lists the .text section with a real digest.
        let text = sections(&base)
            .unwrap()
            .into_iter()
            .find(|section| section.name == ".text")
            .unwrap();
        assert!(text.size > 0);
        assert_eq!(text.sha256.len(), 64);
    }
}
