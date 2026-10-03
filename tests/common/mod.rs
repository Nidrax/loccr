//! Hermetic temporary git repositories driven by the `git` CLI.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// A temporary git repository (plus an optional bare "origin" remote).
pub struct TestRepo {
    /// Keeps the temporary directory alive.
    _dir: TempDir,
    root: PathBuf,
    home: PathBuf,
    tick: std::cell::Cell<u64>,
}

impl TestRepo {
    /// `git init -b trunk` in a fresh temporary directory.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().join("home");
        let root = dir.path().join("repo");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        let repo = Self {
            _dir: dir,
            root,
            home,
            tick: std::cell::Cell::new(0),
        };
        repo.git(&["init", "-q", "-b", "trunk"]);
        repo.git(&["config", "user.name", "Test User"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Parent directory holding the repo, the home dir and any extra repos.
    pub fn sandbox(&self) -> &Path {
        self._dir.path()
    }

    /// A `git` command with a hermetic environment and deterministic dates.
    pub fn command_in(&self, cwd: &Path) -> Command {
        let n = self.tick.get() + 1;
        self.tick.set(n);
        let date = format!("2026-01-01T00:{:02}:{:02}+00:00", (n / 60) % 60, n % 60);
        let mut cmd = Command::new("git");
        cmd.current_dir(cwd)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Test User")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test User")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .env("LC_ALL", "C");
        cmd
    }

    /// Runs `git <args>` in the repo and returns stdout. Panics on failure.
    pub fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.root, args)
    }

    pub fn git_in(&self, cwd: &Path, args: &[&str]) -> String {
        let out = self.command_in(cwd).args(args).output().expect("run git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Writes a file (creating parent directories) relative to the repo root.
    pub fn write(&self, rel: &str, contents: impl AsRef<[u8]>) {
        let p = self.root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, contents).unwrap();
    }

    pub fn remove(&self, rel: &str) {
        std::fs::remove_file(self.root.join(rel)).unwrap();
    }

    /// Stages everything and commits; returns the new commit hash.
    pub fn commit_all(&self, message: &str) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
        self.head()
    }

    pub fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_owned()
    }

    pub fn checkout_new_branch(&self, name: &str) {
        self.git(&["checkout", "-q", "-b", name]);
    }

    /// Creates a bare repository next to the repo, adds it as `origin`, pushes the
    /// current branch with upstream tracking and sets `origin/HEAD`.
    pub fn add_remote(&self) -> PathBuf {
        let bare = self.sandbox().join("origin.git");
        self.git_in(
            self.sandbox(),
            &["init", "-q", "--bare", "-b", "trunk", "origin.git"],
        );
        self.git(&["remote", "add", "origin", bare.to_str().unwrap()]);
        self.git(&["push", "-q", "-u", "origin", "trunk"]);
        self.git(&["remote", "set-head", "origin", "trunk"]);
        bare
    }

    /// Adds a linked worktree for a new branch and returns its path.
    pub fn add_worktree(&self, name: &str) -> PathBuf {
        let path = self.sandbox().join(name);
        self.git(&["worktree", "add", "-q", "-b", name, path.to_str().unwrap()]);
        path
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}
