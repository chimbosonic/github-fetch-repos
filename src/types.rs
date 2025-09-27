use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case, reason = "Follows gh ouput json spec")]
pub struct GHOuput {
    pub sshUrl: String,
}

#[derive(Debug, PartialEq)]
pub struct Repo {
    pub ssh_url: RepoSshUrl,
    pub name: RepoName,
}

pub type RepoName = String;
pub type RepoSshUrl = String;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(
        short,
        long,
        default_value = "chimbosonic",
        help = "GitHub organization"
    )]
    pub github_org: String,

    #[arg(short, long, help = "Perform a dry run without making any changes")]
    pub dry_run: bool,

    #[arg(
        short,
        long,
        help = "List of repo name filters to exclude",
        value_delimiter = ','
    )]
    pub filters: Option<Vec<String>>,
}
