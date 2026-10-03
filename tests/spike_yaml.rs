//! M0 spike 2: the exact review schema round-trips through `yaml_serde`.
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Review {
    version: u32,
    id: String,
    branch: Option<String>,
    head: String,
    origin: String,
    uncommitted: bool,
    created: String,
    updated: String,
    files: Vec<File>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct File {
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    old_path: Option<String>,
    reviewed: bool,
    blob: Option<String>,
    threads: Vec<Thread>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Thread {
    id: String,
    resolved: bool,
    outdated: bool,
    side: String,
    start: Pos,
    end: Pos,
    anchor: String,
    messages: Vec<Message>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Pos {
    line: u32,
    column: u32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Message {
    author: String,
    created: String,
    body: String,
}

fn sample() -> Review {
    Review {
        version: 1,
        id: "20261003T141500Z-a3f9".into(),
        branch: None,
        head: "6cff49933b1e5b0361298993a89b0725501a2039".into(),
        origin: "86eef00fde5f36ebc8f84f9234d918136ef382cc".into(),
        uncommitted: true,
        created: "2026-10-03T14:15:00Z".into(),
        updated: "2026-10-03T14:42:10Z".into(),
        files: vec![File {
            path: "src/file.cpp".into(),
            old_path: Some("src/old.cpp".into()),
            reviewed: false,
            blob: None,
            threads: vec![Thread {
                id: "c3a9f1".into(),
                resolved: false,
                outdated: false,
                side: "new".into(),
                start: Pos {
                    line: 67,
                    column: 1,
                },
                end: Pos {
                    line: 70,
                    column: 13,
                },
                // Tricky strings: leading spaces, trailing newline, CR, tab, quotes,
                // YAML-significant characters, unicode.
                anchor: "  int x = 1;\r\n\tfoo(x) ; # \"q\": 'z' - ~ | > & * ! %\n日本語 😀\n"
                    .into(),
                messages: vec![
                    Message {
                        author: "reviewer".into(),
                        created: "2026-10-03T14:20:00.123Z".into(),
                        body: "Line one\n\nLine three with `code`: yes\n  indented\n".into(),
                    },
                    Message {
                        author: "committer".into(),
                        created: "2026-10-03T14:50:00.000Z".into(),
                        body: "null".into(),
                    },
                ],
            }],
        }],
    }
}

#[test]
fn round_trips_tricky_strings_and_null_branch() {
    let review = sample();
    let text = yaml_serde::to_string(&review).unwrap();
    println!("{text}");
    assert!(
        text.contains("branch: null"),
        "branch must serialise as null:\n{text}"
    );
    let back: Review = yaml_serde::from_str(&text).unwrap();
    assert_eq!(back, review);
}

#[test]
fn old_path_is_omitted_when_absent_and_output_is_deterministic() {
    let mut review = sample();
    review.files[0].old_path = None;
    let a = yaml_serde::to_string(&review).unwrap();
    let b = yaml_serde::to_string(&review).unwrap();
    assert_eq!(a, b);
    assert!(!a.contains("old_path"));
    // Field order follows the struct order.
    assert!(a.find("version").unwrap() < a.find("files").unwrap());
}

#[test]
fn parses_hand_written_spec_example() {
    let text = r#"---
version: 1
id: 20261003T141500Z-a3f9
branch: feature/x
head: 6cff49933b1e5b0361298993a89b0725501a2039
origin: 86eef00fde5f36ebc8f84f9234d918136ef382cc
uncommitted: true
created: 2026-10-03T14:15:00Z
updated: 2026-10-03T14:42:10Z
files:
- path: "src/file.cpp"
  reviewed: true
  blob: 5d2c1e0c8a4b3f7e9a61d0b2c3e4f5a6b7c8d9e0
  threads:
  - id: c3a9f1
    resolved: false
    outdated: false
    side: new
    start: {line: 67, column: 1}
    end: {line: 70, column: 13}
    anchor: "  int x = 1;\n  foo(x) ;\n"
    messages:
    - author: reviewer
      created: 2026-10-03T14:20:00.000Z
      body: "Fix formatting"
"#;
    let review: Review = yaml_serde::from_str(text).unwrap();
    assert_eq!(review.branch.as_deref(), Some("feature/x"));
    assert_eq!(
        review.files[0].threads[0].anchor,
        "  int x = 1;\n  foo(x) ;\n"
    );
}
