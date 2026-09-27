const WORKFLOW: &str = include_str!("../.github/workflows/pr-build.yml");

/// The workflow as checked in uses LF; a Windows checkout may hand us CRLF,
/// which would make the newline-anchored matches below fail. Normalize.
fn workflow() -> String {
    WORKFLOW.replace("\r\n", "\n")
}

#[test]
fn pr_build_comment_job_uses_pull_request_write_permission() {
    let workflow = workflow();
    let (_, comment_job) = workflow
        .split_once("\n  comment:\n")
        .expect("PR build workflow must contain a comment job");

    assert!(
        comment_job.contains("\n      pull-requests: write\n"),
        "the PR comment job must request pull-requests: write"
    );
    assert!(
        !comment_job.contains("\n      issues: write\n"),
        "the PR comment job must not rely on issues: write"
    );
}

#[test]
fn pr_build_avoids_actions_with_node20_runtimes() {
    let workflow = workflow();
    for deprecated in [
        "actions/github-script@v7",
        "actions/checkout@v4",
        "actions/setup-node@v4",
        "actions/cache/restore@v4",
        "actions/upload-artifact@v4",
        "arduino/setup-protoc@v3",
    ] {
        assert!(
            !workflow.contains(deprecated),
            "{deprecated} still uses the deprecated Node 20 action runtime"
        );
    }
}
