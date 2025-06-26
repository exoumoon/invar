pub mod models;

/// A struct that represents the remote [modrinth](https://modrinth.com) repository.
#[derive(Debug)]
#[must_use]
pub struct ModrinthRepository {
    client: reqwest::blocking::Client,
}

impl Default for ModrinthRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl ModrinthRepository {
    pub const USER_AGENT: &str = concat!(
        "https://github.com/exoumoon/invar/",
        env!("CARGO_PKG_VERSION"),
        " (mxxntype)"
    );

    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .user_agent(Self::USER_AGENT)
                .build()
                .expect("Failed to build a Reqwest Client with custom user agent"),
        }
    }

    pub fn fetch_project<S>(&self, project_id: S) -> Result<models::Project, reqwest::Error>
    where
        S: AsRef<str>,
    {
        let project_id = project_id.as_ref();
        let url = format!("https://api.modrinth.com/v3/project/{project_id}");
        let project = self
            .client
            .get(url)
            .send()
            .unwrap()
            .json::<models::Project>()
            .unwrap();
        Ok(project)
    }

    pub fn fetch_versions<S>(&self, project_id: S) -> Result<Vec<models::Version>, reqwest::Error>
    where
        S: AsRef<str>,
    {
        let project_id = project_id.as_ref();
        let url = format!("https://api.modrinth.com/v3/project/{project_id}/version");
        let version = self.client.get(url).send().unwrap().json().unwrap();
        Ok(version)
    }
}
