use std::ops::{Deref, DerefMut};

use git2::IndexAddOption;

pub struct GitRepository {
    raw_repository: git2::Repository,
}

#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("Failed to interact with the underlying Git repository")]
    Git2(#[from] git2::Error),
}

pub type Result<T> = std::result::Result<T, GitError>;

impl From<git2::Repository> for GitRepository {
    fn from(raw_repository: git2::Repository) -> Self {
        Self { raw_repository }
    }
}

impl Deref for GitRepository {
    type Target = git2::Repository;

    fn deref(&self) -> &Self::Target {
        &self.raw_repository
    }
}

impl DerefMut for GitRepository {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw_repository
    }
}

impl GitRepository {
    pub fn stage_all_changes(&self) -> Result<()> {
        let mut index = self.raw_repository.index()?;
        let pathspecs = std::iter::once("*");
        let options = IndexAddOption::DEFAULT;
        let callback = None;

        index.add_all(pathspecs, options, callback)?;
        index.write()?;

        Ok(())
    }
}
