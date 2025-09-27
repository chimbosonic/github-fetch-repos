use clap::Parser;
use futures::stream::{self, StreamExt};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::process::Command;
use tokio::sync::Semaphore;

mod types;
use crate::types::*;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("🔍 Fetching list of repos...");
    let repos = get_list_of_repos(&args.github_org).await?;
    let mut repos = filter_repos(repos, args.filters);
    if args.https {
        repos
            .iter_mut()
            .for_each(|repo| repo.method = RepoMethod::Https);
    }
    let total = repos.len();

    if args.dry_run {
        println!("Dry run mode enabled. The following repositories would be processed:");
        for repo in &repos {
            println!(" - {}: {}", repo.name, repo.url());
        }
        println!("Total repositories to be processed: {total}");
        return Ok(());
    }

    let max_threads: usize = args.max_threads;
    if max_threads >= 10 {
        return Err("Please use less than 10 threads".into());
    };

    let done_counter = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(max_threads));

    println!("🚀 Starting to process {total} repos with max {max_threads} concurrent jobs...");

    stream::iter(
        repos
            .into_iter()
            .map(|repo| process_repo(semaphore.clone(), done_counter.clone(), repo, total)),
    )
    .buffer_unordered(max_threads)
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

    let name = &repo.name;

    if Path::new(name).exists() {
        println!("[{name}] already exists, fetching...");
        let _ = repo.fetch().await.map_err(|err| println!("{err}"));
    } else {
        println!("Cloning [{name}]...");
        let _ = repo.clone().await.map_err(|err| println!("{err}"));
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

impl Repo {
    fn url(&self) -> String {
        match self.method {
            RepoMethod::Https => self.https_url.clone(),
            RepoMethod::Ssh => self.ssh_url.clone(),
        }
    }

    async fn fetch(&self) -> Result<()> {
        let name = &self.name;
        let output = Command::new("git")
            .args(["-C", name, "fetch", "--all"])
            .status()
            .await;

        match output {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => {
                Err(format!("git fetch failed for {name} (code: {:?})", status.code()).into())
            }
            Err(err) => Err(format!("failed to run git fetch for {name}: {err}").into()),
        }
    }

    async fn clone(&self) -> Result<()> {
        let name = &self.name;
        let url = &self.url();

        let output = Command::new("git").args(["clone", url]).status().await;

        match output {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => {
                Err(format!("git clone failed for {name} (code: {:?})", status.code()).into())
            }
            Err(err) => Err(format!("failed to run git clone for {name}: {err}").into()),
        }
    }
}

async fn get_list_of_repos(github_org: &str) -> Result<Vec<Repo>> {
    let output = match Command::new("gh")
        .args([
            "repo", "list", github_org, "--json", "sshUrl", "--json", "url", "-L", "1000",
        ])
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
            https_url: format!("{}.git", value.url),
            name,
            method: RepoMethod::Ssh,
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
            .filter(|repo| !check_filter(repo, &custom_filters))
            .collect();
    }

    repos
}

fn check_filter(repo: &Repo, filters: &Vec<String>) -> bool {
    let name = repo.name.to_lowercase();
    for filter in filters {
        let filter = filter.to_lowercase();

        if name.contains(&filter) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests;
