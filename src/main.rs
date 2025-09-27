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
use tokio::task::JoinHandle;

mod types;
use crate::types::*;

static MAX_THREADS: usize = 5;

#[tokio::main]
async fn main() -> Result<()> {
    let done_counter = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(MAX_THREADS));

    let args = Args::parse();

    println!("🔍 Fetching list of repos...");
    let repos = get_list_of_repos(&args.github_org).await?;
    let repos = filter_repos(repos, args.filters);
    let total = repos.len();

    if args.dry_run {
        println!("Dry run mode enabled. The following repositories would be processed:");
        for repo in &repos {
            println!(" - {}", repo.name);
        }
        println!("Total repositories to be processed: {total}");
        return Ok(());
    }

    println!("🚀 Starting to process {total} repos with max {MAX_THREADS} concurrent jobs...");

    stream::iter(
        repos
            .into_iter()
            .map(|repo| process_repo(semaphore.clone(), done_counter.clone(), repo, total)),
    )
    .buffer_unordered(MAX_THREADS)
    .collect::<Vec<_>>()
    .await;

    println!("🎉 All {total} repos processed!");
    Ok(())
}

async fn process_repo(
    semaphore: Arc<Semaphore>,
    done_counter: Arc<AtomicUsize>,
    repo: Repo,
    repo_total: usize,
) -> () {
        let _permit = semaphore.acquire().await.unwrap();

        let name = repo.name;

        if Path::new(&name).exists() {
            println!("[{name}] already exists, fetching...");
            git_fetch(&name).await;
        } else {
            println!("Cloning [{name}]...");
            git_clone(&repo.ssh_url).await;
        }

        let finished = done_counter.fetch_add(1, Ordering::SeqCst) + 1;
        println!("✅ [{finished}/{repo_total}] Finished {name}");
    
}

fn get_repo_name(ssh_url: &RepoSshUrl) -> Result<String> {
    Ok(ssh_url
        .split('/')
        .next_back()
        .ok_or("Failed to get repo name")?
        .trim_end_matches(".git")
        .to_string())
}

async fn git_fetch(name: &RepoName) -> () {
    let _ = Command::new("git")
        .args(["-C", name, "fetch", "--all"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;
}

async fn git_clone(ssh_url: &RepoSshUrl) -> () {
    let _ = Command::new("git")
        .args(["clone", ssh_url])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;
}

async fn get_list_of_repos(github_org: &str) -> Result<Vec<Repo>> {
    let output = match Command::new("gh")
        .args(["repo", "list", github_org, "--json", "sshUrl", "-L", "1000"])
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) => {
            return Err(format!("Failed to execute gh command: {e}").into());
        }
    };

    if !output.status.success() {
        return Err("gh command failed".into());
    }

    parse_gh_output(&output.stdout)
}

impl TryFrom<&GHOuput> for Repo {
    type Error = Error;

    fn try_from(value: &GHOuput) -> Result<Repo> {
        let name = get_repo_name(&value.sshUrl)?;

        Ok(Self {
            ssh_url: value.sshUrl.clone(),
            name,
        })
    }
}

fn parse_gh_output(output: &[u8]) -> Result<Vec<Repo>> {
    let repos: Vec<GHOuput> = serde_json::from_slice(output)?;
    repos.iter().map(Repo::try_from).collect()
}

fn filter_repos(repos: Vec<Repo>, filters: Option<Vec<String>>) -> Vec<Repo> {
    if let Some(custom_filters) = filters {
        return repos
            .into_iter()
            .filter(|repo| !check_filter(&repo.ssh_url, &custom_filters))
            .collect();
    }

    repos
}

fn check_filter(ssh_url: &RepoSshUrl, filters: &Vec<String>) -> bool {
    for filter in filters {
        if ssh_url.contains(filter) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests;
