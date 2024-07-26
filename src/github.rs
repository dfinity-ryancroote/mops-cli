use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoInfo {
    pub repo: String,
    pub tag: String,
    pub commit: String,
}

/// Parse github url as specified in `https://docs.mops.one/mops.toml`
pub async fn parse_github_url(url: &str) -> Result<RepoInfo> {
    // https://github.com/icdevsorg/candy_library#v0.3.0@907a4e7363aac6c6a4e114ebc73e3d3f21e138af
    // or https://github.com/chenyan2002/motoko-splay.git
    let url = url
        .strip_prefix("https://github.com/")
        .ok_or_else(|| anyhow::anyhow!("invalid url"))?;
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!("invalid url"));
    }
    let owner = parts[0];
    let mut repo_part = parts[1];
    if repo_part.ends_with(".git") {
        repo_part = repo_part.strip_suffix(".git").unwrap();
    }
    let repo_parts: Vec<&str> = repo_part.split('#').collect();
    let repo = format!("{}/{}", owner, repo_parts[0]);
    let mut tag = None;
    let mut commit = None;
    if repo_parts.len() > 1 {
        let tag_commit_parts: Vec<&str> = repo_parts[1].split('@').collect();
        tag = Some(tag_commit_parts[0].to_string());
        if tag_commit_parts.len() > 1 {
            commit = Some(tag_commit_parts[1].to_string());
        }
    }
    if tag.is_none() {
        tag = Some(get_default_branch(&repo).await?);
    }
    if commit.is_none() {
        commit = Some(get_latest_commit(&repo, tag.as_ref().unwrap()).await?);
    }
    Ok(RepoInfo {
        repo,
        tag: tag.unwrap(),
        commit: commit.unwrap(),
    })
}

async fn get_default_branch(repo: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Branch {
        default_branch: String,
    }
    let url = format!("https://api.github.com/repos/{}", repo);
    let body = github_request(&url).await?;
    let response = serde_json::from_str::<Branch>(&body).map_err(|_| anyhow::anyhow!("{body}"))?;
    Ok(response.default_branch)
}

async fn get_latest_commit(repo: &str, tag: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Commit {
        sha: String,
    }
    let url = format!("https://api.github.com/repos/{}/commits/{}", repo, tag);
    let body = github_request(&url).await?;
    let response = serde_json::from_str::<Commit>(&body).map_err(|_| anyhow::anyhow!("{body}"))?;
    Ok(response.sha)
}

pub async fn fetch_file(repo: &RepoInfo, file: &str) -> Result<String> {
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}",
        repo.repo, repo.commit, file
    );
    let body = github_request(&url).await?;
    if body.starts_with("404: Not Found") {
        return Err(anyhow::anyhow!("file not found"));
    }
    Ok(body)
}

async fn github_request(url: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let mut request = client.get(url).header("User-Agent", "mops-cli");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let response = request.send().await?;
    let body = response.text().await?;
    Ok(body)
}