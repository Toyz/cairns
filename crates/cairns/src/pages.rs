//! Where the site will be published, when `site.base_url` does not say.
//!
//! A project on GitHub or GitLab Pages is published at an address that
//! follows from where the repository is, so it can be worked out rather than
//! left empty - an empty one left `ids.json`, the feed and every page's
//! canonical link without an address. A custom domain cannot be guessed, and
//! a project using one says so in `base_url`.

use std::path::Path;

/// The Pages address, and what it was worked out from - in order of how sure
/// each is: what GitLab's CI says outright, what GitHub's CI says the
/// repository is, the project's `repository`, then this checkout's `origin`.
pub fn infer(root: &Path, repository: Option<&str>) -> Option<(String, String)> {
    if let Some(url) = env("CI_PAGES_URL") {
        return Some((slashed(&url), "GitLab CI's CI_PAGES_URL".into()));
    }
    if let Some(path) = env("GITHUB_REPOSITORY")
        && let Some(url) = from_host_and_path("github.com", &path)
    {
        return Some((url, "GitHub Actions' GITHUB_REPOSITORY".into()));
    }
    if let Some(path) = env("CI_PROJECT_PATH")
        && let Some(url) = from_host_and_path("gitlab.com", &path)
    {
        return Some((url, "GitLab CI's CI_PROJECT_PATH".into()));
    }
    if let Some(url) = repository.and_then(from_remote) {
        return Some((url, "project.repository".into()));
    }
    let origin = origin(root)?;
    from_remote(&origin).map(|url| (url, "the git remote origin".into()))
}

/// The repository's web address, when `project.repository` does not say -
/// from CI, or this checkout's `origin` - for GitHub and GitLab, whose
/// addresses follow from the path. What a README's links and the license
/// files are linked into.
pub fn infer_repository(root: &Path) -> Option<String> {
    if let Some(url) = env("CI_PROJECT_URL") {
        return Some(url.trim_end_matches('/').to_string());
    }
    if let Some(path) = env("GITHUB_REPOSITORY") {
        let server = env("GITHUB_SERVER_URL").unwrap_or_else(|| "https://github.com".into());
        return Some(format!("{}/{path}", server.trim_end_matches('/')));
    }
    web_address(&origin(root)?)
}

/// A remote's URL - `git@github.com:Owner/repo.git` - as the repository's web
/// address, `https://github.com/Owner/repo`, for GitHub and GitLab.
pub fn web_address(remote: &str) -> Option<String> {
    let remote = remote.trim();
    let (host, path) = if let Some(rest) = remote.strip_prefix("git@") {
        rest.split_once(':')?
    } else {
        let rest = remote.split_once("://")?.1;
        let rest = rest.rsplit_once('@').map(|(_, r)| r).unwrap_or(rest);
        rest.split_once('/')?
    };
    let host = host.to_ascii_lowercase();
    if host != "github.com" && host != "gitlab.com" {
        return None;
    }
    let path = path.trim_matches('/').trim_end_matches(".git");
    (path.split('/').count() >= 2).then(|| format!("https://{host}/{path}"))
}

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn slashed(url: &str) -> String {
    format!("{}/", url.trim().trim_end_matches('/'))
}

/// The origin remote's URL, read in-process.
fn origin(root: &Path) -> Option<String> {
    let repo = gix::discover(root).ok()?;
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url(gix::remote::Direction::Fetch)?;
    Some(url.to_bstring().to_string())
}

/// A repository URL - `https://github.com/Owner/repo(.git)`,
/// `git@gitlab.com:group/sub/project.git` - as its Pages address, for the two
/// hosts whose Pages addresses follow from the path.
pub fn from_remote(remote: &str) -> Option<String> {
    let remote = remote.trim();
    let (host, path) = if let Some(rest) = remote.strip_prefix("git@") {
        rest.split_once(':')?
    } else {
        let rest = remote.split_once("://")?.1;
        let rest = rest.rsplit_once('@').map(|(_, r)| r).unwrap_or(rest);
        rest.split_once('/')?
    };
    from_host_and_path(host, path)
}

/// `owner.github.io/repo/`, or `group.gitlab.io/sub/project/`; the root of
/// the domain for a repository named after it - `owner.github.io` itself.
fn from_host_and_path(host: &str, path: &str) -> Option<String> {
    let domain = match host.to_ascii_lowercase().as_str() {
        "github.com" => "github.io",
        "gitlab.com" => "gitlab.io",
        _ => return None,
    };
    let path = path.trim_matches('/').trim_end_matches(".git");
    let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 || (domain == "github.io" && parts.len() != 2) {
        return None;
    }
    let owner = parts.remove(0).to_ascii_lowercase();
    let site = format!("{owner}.{domain}");
    let rest = parts.join("/");
    if rest.eq_ignore_ascii_case(&site) {
        return Some(format!("https://{site}/"));
    }
    Some(format!("https://{site}/{rest}/"))
}

#[cfg(test)]
mod tests {
    use super::from_remote;

    #[test]
    fn a_pages_address_follows_from_where_the_repository_is() {
        assert_eq!(
            from_remote("https://github.com/Toyz/piney_apples").as_deref(),
            Some("https://toyz.github.io/piney_apples/")
        );
        assert_eq!(
            from_remote("git@github.com:Toyz/cairns.git").as_deref(),
            Some("https://toyz.github.io/cairns/")
        );
        assert_eq!(
            from_remote("https://github.com/Toyz/toyz.github.io.git").as_deref(),
            Some("https://toyz.github.io/")
        );
        assert_eq!(
            from_remote("https://gitlab.com/Group/sub/project.git").as_deref(),
            Some("https://group.gitlab.io/sub/project/")
        );
        assert_eq!(
            from_remote("git@gitlab.com:group/group.gitlab.io.git").as_deref(),
            Some("https://group.gitlab.io/")
        );
        assert_eq!(
            from_remote("https://user:token@github.com/Toyz/cairns.git").as_deref(),
            Some("https://toyz.github.io/cairns/")
        );
        assert_eq!(from_remote("https://codeberg.org/x/y"), None);
        assert_eq!(
            super::web_address("git@github.com:Toyz/piney_apples.git").as_deref(),
            Some("https://github.com/Toyz/piney_apples")
        );
        assert_eq!(
            super::web_address("https://user:t@gitlab.com/g/sub/p.git").as_deref(),
            Some("https://gitlab.com/g/sub/p")
        );
        assert_eq!(from_remote("not a url"), None);
    }
}
