//! M0 spike 1: can `gix` diff "origin tree" against "HEAD + index + worktree + untracked"
//! with rename/copy detection, matching `git diff -M -C`?
//!
//! Approach: build a virtual new-side tree in memory (`with_object_memory`) by editing the
//! HEAD tree with the current worktree content of every path `gix status` reports, then run
//! a tree-to-tree diff with rewrite tracking.
mod common;

use std::collections::BTreeSet;

use common::TestRepo;
use gix::bstr::{BString, ByteSlice};
use gix::diff::tree_with_rewrites::Change;
use gix::object::tree::EntryKind;

/// Normalised `STATUS\tpath[\tpath]` lines (similarity score dropped).
fn gix_changes(path: &std::path::Path, base: &str, copies: bool) -> BTreeSet<String> {
    let repo = gix::open(path).unwrap().with_object_memory();
    let workdir = repo.workdir().unwrap().to_owned();

    // 1. Paths that differ from HEAD anywhere (staged, unstaged, untracked).
    let mut paths: BTreeSet<BString> = BTreeSet::new();
    let status = repo
        .status(gix::progress::Discard)
        .unwrap()
        .untracked_files(gix::status::UntrackedFiles::Files)
        .index_worktree_rewrites(None);
    for item in status.into_iter(Vec::<BString>::new()).unwrap() {
        let item = item.unwrap();
        paths.insert(item.location().to_owned());
    }

    // 2. Virtual new-side tree: HEAD tree + worktree content of those paths.
    let head_tree = repo.head_commit().unwrap().tree().unwrap();
    let mut editor = repo.edit_tree(head_tree.id).unwrap();
    for rela in &paths {
        let abs = workdir.join(rela.to_str().unwrap());
        match std::fs::symlink_metadata(&abs) {
            Ok(meta) if meta.is_file() => {
                let id = repo
                    .write_blob(std::fs::read(&abs).unwrap())
                    .unwrap()
                    .detach();
                #[cfg(unix)]
                let kind = {
                    use std::os::unix::fs::PermissionsExt;
                    if meta.permissions().mode() & 0o111 != 0 {
                        EntryKind::BlobExecutable
                    } else {
                        EntryKind::Blob
                    }
                };
                editor.upsert(rela.as_bstr(), kind, id).unwrap();
            }
            Ok(meta) if meta.file_type().is_symlink() => {
                let target = std::fs::read_link(&abs).unwrap();
                let id = repo
                    .write_blob(target.to_str().unwrap().as_bytes())
                    .unwrap()
                    .detach();
                editor.upsert(rela.as_bstr(), EntryKind::Link, id).unwrap();
            }
            Ok(_) => {} // directories / nested repos: ignored in the spike
            Err(_) => {
                editor.remove(rela.as_bstr()).unwrap();
            }
        }
    }
    let new_tree_id = editor.write().unwrap();
    let new_tree = repo.find_tree(new_tree_id).unwrap();

    // 3. Diff base tree -> virtual tree with rewrite tracking.
    let base_tree = repo
        .rev_parse_single(base)
        .unwrap()
        .object()
        .unwrap()
        .peel_to_tree()
        .unwrap();
    let mut opts = gix::diff::Options::default();
    opts.track_path();
    opts.track_rewrites(Some(gix::diff::Rewrites {
        copies: copies.then(Default::default),
        percentage: Some(0.5),
        limit: 1000,
        track_empty: false,
    }));
    let changes = repo.diff_tree_to_tree(&base_tree, &new_tree, opts).unwrap();
    if std::env::var_os("SPIKE_DEBUG").is_some() {
        eprintln!("{changes:#?}");
    }

    // Workaround for a gix/git difference: a file used as a *copy source* loses its own
    // `Modification` entry. Re-add it when the source differs between the two trees.
    let mut extra: BTreeSet<String> = BTreeSet::new();
    for c in &changes {
        if let Change::Rewrite {
            source_location,
            copy: true,
            ..
        } = c
        {
            let old = base_tree
                .lookup_entry_by_path(source_location.to_str().unwrap())
                .unwrap();
            let new = new_tree
                .lookup_entry_by_path(source_location.to_str().unwrap())
                .unwrap();
            if let (Some(old), Some(new)) = (old, new)
                && old.oid() != new.oid()
            {
                extra.insert(format!("M\t{source_location}"));
            }
        }
    }

    changes
        .into_iter()
        .filter_map(|c| match c {
            Change::Addition {
                location,
                entry_mode,
                ..
            } if entry_mode.is_blob_or_symlink() => Some(format!("A\t{location}")),
            Change::Deletion {
                location,
                entry_mode,
                ..
            } if entry_mode.is_blob_or_symlink() => Some(format!("D\t{location}")),
            Change::Modification {
                location,
                previous_entry_mode,
                entry_mode,
                ..
            } if entry_mode.is_blob_or_symlink() || previous_entry_mode.is_blob_or_symlink() => {
                Some(format!("M\t{location}"))
            }
            Change::Rewrite {
                source_location,
                location,
                copy,
                ..
            } => Some(format!(
                "{}\t{source_location}\t{location}",
                if copy { "C" } else { "R" }
            )),
            _ => None,
        })
        .chain(extra)
        .collect()
}

/// Oracle: stage everything into a throw-away index, write the tree into a quarantine
/// object dir (so the repo's object database is not polluted) and run `git diff-tree`.
fn git_changes(repo: &TestRepo, base: &str, copies: bool) -> BTreeSet<String> {
    let tmp = tempfile::tempdir().unwrap();
    let index = tmp.path().join("index");
    let objects = tmp.path().join("objects");
    std::fs::create_dir(&objects).unwrap();
    let git_dir = repo
        .git(&["rev-parse", "--absolute-git-dir"])
        .trim()
        .to_owned();
    std::fs::copy(format!("{git_dir}/index"), &index).ok();

    let run = |args: &[&str]| {
        let mut cmd = repo.command_in(repo.path());
        cmd.env("GIT_INDEX_FILE", &index)
            .env("GIT_OBJECT_DIRECTORY", &objects)
            .env(
                "GIT_ALTERNATE_OBJECT_DIRECTORIES",
                format!("{git_dir}/objects"),
            )
            .args(args);
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };
    run(&["add", "-A"]);
    let tree = run(&["write-tree"]).trim().to_owned();
    let mut args = vec!["diff-tree", "-r", "-M", "--name-status"];
    if copies {
        args.push("-C");
    }
    args.extend([base, &tree]);
    run(&args)
        .lines()
        .map(|l| {
            // "R100\told\tnew" -> "R\told\tnew"
            let (status, rest) = l.split_once('\t').unwrap();
            format!("{}\t{rest}", &status[..1])
        })
        .collect()
}

fn lines(prefix: &str, n: usize) -> String {
    (0..n)
        .map(|i| format!("{prefix} line number {i}\n"))
        .collect()
}

#[test]
fn worktree_diff_matches_git_for_renames_and_untracked() {
    let repo = TestRepo::new();
    repo.write("keep.txt", lines("keep", 30));
    repo.write("rename_me.txt", lines("rename", 30));
    repo.write("rename_edit.txt", lines("redit", 40));
    repo.write("delete_me.txt", lines("delete", 30));
    repo.write("modify.txt", lines("modify", 30));
    repo.write("staged_mod.txt", lines("staged", 30));
    repo.write("src/orig.rs", lines("orig", 50));
    let base = repo.commit_all("base");

    // committed: a rename + edit
    repo.git(&["mv", "rename_edit.txt", "renamed_edit.txt"]);
    repo.write("renamed_edit.txt", format!("{}extra\n", lines("redit", 40)));
    repo.commit_all("committed rename");

    // unstaged modify, staged modify, deleted in worktree only
    repo.write("modify.txt", format!("{}tail\n", lines("modify", 30)));
    repo.write("staged_mod.txt", format!("{}tail\n", lines("staged", 30)));
    repo.git(&["add", "staged_mod.txt"]);
    repo.remove("delete_me.txt");
    // worktree rename of a tracked file without staging: untracked new file + deleted old
    repo.remove("rename_me.txt");
    repo.write("moved/rename_me.txt", lines("rename", 30));
    // untracked new file, ignored file
    repo.write("brand_new.txt", "new file\n");
    repo.write(".gitignore", "ignored.log\n");
    repo.write("ignored.log", "ignored\n");

    let expected = git_changes(&repo, &base, false);
    let actual = gix_changes(repo.path(), &base, false);
    println!("expected: {expected:#?}\nactual: {actual:#?}");
    assert!(
        expected.iter().any(|l| l.starts_with("R\trename_me.txt")),
        "oracle sanity"
    );
    assert_eq!(actual, expected);
}

#[test]
fn worktree_diff_matches_git_for_copies() {
    let repo = TestRepo::new();
    repo.write("src/orig.rs", lines("orig", 50));
    repo.write("other.txt", lines("other", 20));
    let base = repo.commit_all("base");

    // copy of an *unmodified* file only detected with --find-copies-harder; copy of a modified
    // file detected with plain -C.
    repo.write("src/orig.rs", format!("{}more\n", lines("orig", 50)));
    repo.write("src/copy.rs", lines("orig", 50));
    repo.write("src/harder.rs", lines("other", 20));

    let expected = git_changes(&repo, &base, true);
    let actual = gix_changes(repo.path(), &base, true);
    println!("expected: {expected:#?}\nactual: {actual:#?}");
    assert!(
        expected.iter().any(|l| l.starts_with("C\t")),
        "oracle sanity"
    );
    assert_eq!(actual, expected);
}

#[test]
fn does_not_pollute_the_object_database() {
    let repo = TestRepo::new();
    repo.write("a.txt", "a\n");
    let base = repo.commit_all("base");
    repo.write(
        "untracked.txt",
        "this blob must not be written to .git/objects\n",
    );
    let before = repo.git(&["count-objects", "-v"]);
    let _ = gix_changes(repo.path(), &base, false);
    assert_eq!(repo.git(&["count-objects", "-v"]), before);
}

#[test]
fn modification_in_subdir_is_reported_without_copy_tracking() {
    let repo = TestRepo::new();
    repo.write("src/orig.rs", lines("orig", 50));
    let base = repo.commit_all("base");
    repo.write("src/orig.rs", format!("{}more\n", lines("orig", 50)));
    repo.write("src/copy.rs", lines("orig", 50));
    let expected = git_changes(&repo, &base, false);
    let actual = gix_changes(repo.path(), &base, false);
    println!("expected: {expected:#?}\nactual: {actual:#?}");
    assert_eq!(actual, expected);
}
