use super::*;

#[test]
fn test_check_filter() {
    let repo_ssh_url: RepoSshUrl =
        "git@github.com:chimbosonic/hackers.chimbosonic.com.git".to_string();

    assert_eq!(
        check_filter(&repo_ssh_url, &vec!["hackers".to_string()]),
        true
    );
}

#[test]
fn test_get_repo_name() {
    let repo_ssh_url: RepoSshUrl =
        "git@github.com:chimbosonic/hackers.chimbosonic.com.git".to_string();
    assert_eq!(
        get_repo_name(&repo_ssh_url).unwrap(),
        "hackers.chimbosonic.com"
    );
}

#[test]
fn test_parse_gh_output() {
    let data = r#"[{"sshUrl":"git@github.com:chimbosonic/cli-kneeboard.git"},{"sshUrl":"git@github.com:chimbosonic/chimbosonic.com.git"}]"#;

    let repos = parse_gh_output(data.as_bytes()).unwrap();

    assert_eq!(repos.len(), 2);

    assert_eq!(
        repos[0],
        Repo {
            ssh_url: "git@github.com:chimbosonic/cli-kneeboard.git".to_string(),
            name: "cli-kneeboard".to_string()
        }
    );
    assert_eq!(
        repos[1],
        Repo {
            ssh_url: "git@github.com:chimbosonic/chimbosonic.com.git".to_string(),
            name: "chimbosonic.com".to_string()
        }
    );
}

#[test]
fn test_try_from_repo() {
    let gh_output = GHOuput {
        sshUrl: "git@github.com:chimbosonic/cli-kneeboard.git".to_string(),
    };
    let repo: Repo = Repo::try_from(&gh_output).unwrap();

    assert_eq!(
        repo,
        Repo {
            ssh_url: "git@github.com:chimbosonic/cli-kneeboard.git".to_string(),
            name: "cli-kneeboard".to_string()
        }
    )
}
