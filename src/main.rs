use clap::Parser;
use futures::stream::{self, StreamExt};
use std::path::Path;
use std::process::Stdio;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::process::Command;
use tokio::sync::Semaphore;

static MAX_THREADS: usize = 5;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "chimbosonic",
        help = "GitHub organization"
    )]
    github_org: String,

    #[arg(short, long, help = "Perform a dry run without making any changes")]
    dry_run: bool,

    #[arg(
        short,
        long,
        help = "List of repo name filters to exclude",
        value_delimiter = ','
    )]
    filters: Option<Vec<String>>,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let done = Arc::new(AtomicUsize::new(0));
    let sem = Arc::new(Semaphore::new(MAX_THREADS));

    let args = Args::parse();

    println!("🔍 Fetching list of repos...");
    let repos = get_list_of_repos(&args.github_org).await?;
    let repos = filter_repos(repos, args.filters);
    let total = repos.len();

    if args.dry_run {
        println!("Dry run mode enabled. The following repositories would be processed:");
        for repo in &repos {
            let name = get_repo_name(repo);
            println!(" - {name}");
        }
        println!("Total repositories to be processed: {total}");
        return Ok(());
    }

    println!("🚀 Starting to process {total} repos with max {MAX_THREADS} concurrent jobs...");

    stream::iter(repos.into_iter().map(|repo| {
        let done = done.clone();
        let sem = sem.clone();

        tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let name = get_repo_name(&repo);

            if Path::new(&name).exists() {
                println!("[{name}] already exists, fetching...");
                git_fetch(&name).await;
            } else {
                println!("Cloning [{name}]...");
                git_clone(&repo).await;
            }

            let finished = done.fetch_add(1, Ordering::SeqCst) + 1;
            println!("✅ [{finished}/{total}] Finished {name}");
        })
    }))
    .buffer_unordered(MAX_THREADS)
    .collect::<Vec<_>>()
    .await;

    println!("🎉 All {total} repos processed!");
    Ok(())
}

fn get_repo_name(repo: &str) -> String {
    repo.split('/')
        .next_back()
        .unwrap()
        .trim_end_matches(".git")
        .to_string()
}

async fn git_fetch(name: &str) -> () {
    let _ = Command::new("git")
        .args(["-C", name, "fetch", "--all"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;
}

async fn git_clone(repo: &str) -> () {
    let _ = Command::new("git")
        .args(["clone", repo])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;
}

async fn get_list_of_repos(github_org: &str) -> Result<Vec<String>> {
    let output = Command::new("gh")
        .args(["repo", "list", github_org, "--json", "sshUrl", "-L", "1000"])
        .output()
        .await;

    let output = match output {
        Ok(output) => output,
        Err(e) => {
            return Err(format!("Failed to execute gh command: {e}").into());
        }
    };

    if !output.status.success() {
        return Err("gh repo list failed".into());
    }

    let repos: Vec<String> = parse_gh_output(&output.stdout)?;
    Ok(repos)
}

fn parse_gh_output(output: &[u8]) -> Result<Vec<String>> {
    let repos = serde_json::from_slice::<Vec<serde_json::Value>>(output)?
        .into_iter()
        .map(|obj| obj["sshUrl"].as_str().unwrap().to_string())
        .collect();
    Ok(repos)
}

fn filter_repos(repos: Vec<String>, filters: Option<Vec<String>>) -> Vec<String> {
    if let Some(custom_filters) = filters {
        return repos
            .into_iter()
            .filter(|repo| !check_filter(repo, custom_filters.clone()))
            .collect();
    }

    repos
}

fn check_filter(repo: &str, filters: Vec<String>) -> bool {
    for filter in filters {
        if repo.contains(&filter) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_filter() {
        assert_eq!(
            check_filter(
                "git@github.com:chimbosonic/hackers.chimbosonic.com.git",
                vec!["hackers".to_string()]
            ),
            true
        );
    }

    #[test]
    fn test_get_repo_name() {
        assert_eq!(
            get_repo_name("git@github.com:chimbosonic/hackers.chimbosonic.com.git"),
            "hackers.chimbosonic.com"
        );
    }

    #[test]
    fn test_parse_gh_output() {
        let data = r#"[{"sshUrl":"git@github.com:chimbosonic/cli-kneeboard.git"},{"sshUrl":"git@github.com:chimbosonic/chimbosonic.com.git"}]"#;

        let repos = parse_gh_output(data.as_bytes()).unwrap();

        assert_eq!(repos.len(), 2);

        assert_eq!(repos[0], "git@github.com:chimbosonic/cli-kneeboard.git");
        assert_eq!(repos[1], "git@github.com:chimbosonic/chimbosonic.com.git");
    }
}
