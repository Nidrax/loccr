mod common;
use common::TestRepo;

#[test]
fn test_repo_is_hermetic_and_deterministic() {
    let repo = TestRepo::new();
    repo.write("a.txt", "hello\n");
    let h1 = repo.commit_all("first");
    assert_eq!(h1.len(), 40);
    repo.add_remote();
    let wt = repo.add_worktree("wt");
    assert!(wt.join(".git").is_file(), "linked worktree has a .git file");
    assert!(
        repo.git(&["rev-parse", "--abbrev-ref", "origin/HEAD"])
            .contains("origin/trunk")
    );
}
