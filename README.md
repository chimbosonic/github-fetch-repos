# GitHub Fetch repos

Command-line tool written in Rust for bulk cloning and fetching GitHub repositories from an organization.
This tool leverages the GitHub CLI and Git.

## Features

- **Bulk Repository Operations**: Clone all repositories from a GitHub organization or fetch updates for existing ones
- **Concurrent Processing**: Uses async/await
- **Smart Repository Detection**: Automatically detects existing repositories and fetches instead of cloning
- **Filtering Support**: Exclude specific repositories using customizable filters
- **Dry Run Mode**: Preview which repositories would be processed without making changes
- **GitHub CLI Integration**: Uses the official GitHub CLI for authenticated API access

## Prerequisites

- [GitHub CLI](https://cli.github.com/) installed and authenticated
- Git installed and configured

## Installation

### From Source

```bash
git clone <repository-url>
cd github_fetch
cargo install --path .
```

## Usage

### Basic Usage

Clone all repositories from an organization:

```bash
fetch_repos --github-org <organization-name>
```

### Command Line Options

- `-g, --github-org <ORG>`: GitHub organization name (default: "chimbosonic")
- `-d, --dry-run`: Perform a dry run without making any changes
- `-f, --filters <FILTERS>`: Comma-separated list of repository name filters to exclude

### Examples

```bash
# Clone all repositories from 'myorg'
fetch_repos --github-org myorg

# Dry run to see what would be processed
fetch_repos --github-org myorg --dry-run

# Exclude repositories containing 'test' or 'demo' in their names
fetch_repos --github-org myorg --filters test,demo

# Combine options
fetch_repos --github-org myorg --filters archived,legacy --dry-run
```

## How It Works

1. **Repository Discovery**: Uses `gh repo list` to fetch all repositories from the specified organization
2. **Filtering**: Applies any specified filters to exclude unwanted repositories
3. **Smart Operations**:
   - If a directory already exists: runs `git fetch --all`
   - If directory doesn't exist: runs `git clone`
