#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! BED-01 (D-17): the tree digest over a checked-out tree. Unix only, because
//! the reference tree needs an executable bit and symlinks.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

use mt_capture::treehash::{TreeHashError, sha256_hex, tree_rows, tree_sha256};

fn write(root: &Path, relpath: &str, content: &str) {
    let path = root.join(relpath);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// The reference tree: a regular file, an executable file, a symlink, a
/// top-level `.git`, a nested directory and a nested `.git`.
fn reference_tree(root: &Path) {
    write(root, "a.txt", "alpha\n");
    write(root, "bin/run.sh", "#!/bin/sh\n");
    fs::set_permissions(root.join("bin/run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    symlink("a.txt", root.join("link")).unwrap();
    write(root, ".git/config", "ignored\n");
    write(root, "sub/dir/deep.txt", "deep\n");
    write(root, "sub/.git/HEAD", "kept\n");
}

// Expected digests were computed independently of this crate with:
//   printf 'alpha\n'     | sha256sum
//   printf '#!/bin/sh\n' | sha256sum
//   printf 'a.txt'       | sha256sum      (the link target text, no newline)
//   printf 'deep\n'      | sha256sum
//   printf 'kept\n'      | sha256sum
// and the final digest with `sha256sum` over the five rows below, in order,
// each ending in a newline.
const EXPECTED_ROWS: [&str; 5] = [
    "100644 b6a98d9ce9a2d9149288fa3df42d377c3e42737afdcdaf714e33c0a100b51060 a.txt\n",
    "100755 a8076d3d28d21e02012b20eaf7dbf75409a6277134439025f282e368e3305abf bin/run.sh\n",
    "120000 18b7cb099a9ea3f50ba899b5ba81e0d377a5f3b16f8f6eeb8b3e58cd4692b993 link\n",
    "100644 78051faade059d70866df6a3fb83ef348721fd74a87e93ef95c493f87d0d236b sub/.git/HEAD\n",
    "100644 64896f89fd11190013b70103e603a1c5826e56b7fb7d2197ab279b0690043599 sub/dir/deep.txt\n",
];
const EXPECTED_DIGEST: &str = "47405da3aa41dd35b7ddafe544e13169ce90e773726b32fbc506c79623174f3b";

#[test]
fn bed_01_tree_hash_matches_reference_vectors() {
    let dir = tempfile::tempdir().unwrap();
    reference_tree(dir.path());

    let rows = tree_rows(dir.path()).unwrap();
    assert_eq!(rows, EXPECTED_ROWS);
    assert_eq!(tree_sha256(dir.path()).unwrap(), EXPECTED_DIGEST);

    // The digest depends only on the rows: the same content in another place hashes the same.
    let other = tempfile::tempdir().unwrap();
    reference_tree(other.path());
    assert_eq!(tree_sha256(other.path()).unwrap(), EXPECTED_DIGEST);

    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn bed_01_tree_hash_skips_only_top_level_git() {
    let dir = tempfile::tempdir().unwrap();
    reference_tree(dir.path());
    let rows = tree_rows(dir.path()).unwrap();
    assert!(
        !rows.iter().any(|row| row.ends_with(" .git/config\n")),
        "the top-level .git was hashed: {rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.ends_with(" sub/.git/HEAD\n")),
        "a nested .git was skipped: {rows:?}"
    );

    // Adding or changing the top-level .git changes nothing.
    let before = tree_sha256(dir.path()).unwrap();
    write(dir.path(), ".git/objects/pack/new", "more\n");
    assert_eq!(tree_sha256(dir.path()).unwrap(), before);

    // A top-level file called .gitignore is an ordinary file.
    write(dir.path(), ".gitignore", "target\n");
    assert_ne!(tree_sha256(dir.path()).unwrap(), before);
}

#[test]
fn bed_01_tree_hash_does_not_follow_symlinks() {
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "secret.txt", "outside the tree\n");

    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.txt", "alpha\n");
    // A link to a directory and a link to a file, both leaving the tree.
    symlink(outside.path(), dir.path().join("escape_dir")).unwrap();
    symlink(
        outside.path().join("secret.txt"),
        dir.path().join("escape_file"),
    )
    .unwrap();
    // A dangling link is a row too.
    symlink("missing", dir.path().join("dangling")).unwrap();

    let rows = tree_rows(dir.path()).unwrap();
    let names: Vec<&str> = rows
        .iter()
        .map(|row| row.trim_end().rsplit(' ').next().unwrap())
        .collect();
    assert_eq!(names, ["a.txt", "dangling", "escape_dir", "escape_file"]);
    assert!(rows.iter().all(|row| !row.contains("secret.txt ")));

    let link_row = |name: &str| {
        rows.iter()
            .find(|row| row.ends_with(&format!(" {name}\n")))
            .unwrap()
            .clone()
    };
    assert_eq!(
        link_row("dangling"),
        format!("120000 {} dangling\n", sha256_hex(b"missing"))
    );
    assert_eq!(
        link_row("escape_dir"),
        format!(
            "120000 {} escape_dir\n",
            sha256_hex(outside.path().to_str().unwrap().as_bytes())
        )
    );

    // Changing the file behind a link does not change the digest.
    let before = tree_sha256(dir.path()).unwrap();
    write(outside.path(), "secret.txt", "changed\n");
    assert_eq!(tree_sha256(dir.path()).unwrap(), before);
}

#[test]
fn bed_01_tree_hash_rejects_special_files() {
    use std::os::unix::net::UnixListener;

    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.txt", "alpha\n");
    let socket = dir.path().join("sock");
    let _listener = UnixListener::bind(&socket).unwrap();
    match tree_rows(dir.path()) {
        Err(TreeHashError::SpecialFile { path }) => assert_eq!(path, socket),
        other => panic!("expected a SpecialFile error, got {other:?}"),
    }
}
