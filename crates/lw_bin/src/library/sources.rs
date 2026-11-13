//! Source ownership follows the same secure iCloud identity as the shipping Swift app.
use super::{LibraryWorker, db};
use lw_core::{Source, User, hash_string};
use std::io;

pub struct SourceAccount {
    pub user: User,
    pub sources: Vec<Source>,
}

impl LibraryWorker {
    fn user_for_identity(&mut self, encoded: Option<&str>) -> io::Result<User> {
        self.repository
            .get_or_create_user(encoded.map_or(-1, hash_string))
            .map_err(db)
    }

    pub fn current_sources(&mut self) -> io::Result<SourceAccount> {
        let encoded = lw_apple_sys::cloud::current_identity()?;
        let user = self.user_for_identity(encoded.as_deref())?;
        self.sources_for_user(user)
    }

    fn sources_for_user(&mut self, user: User) -> io::Result<SourceAccount> {
        let mut sources = self.repository.sources(user.id).map_err(db)?;
        if !sources.iter().any(|source| source.is_current)
            && let Some(first) = sources.first_mut()
        {
            *first = self
                .repository
                .set_current_source(
                    user.id,
                    first
                        .id
                        .ok_or_else(|| io::Error::other("Source is not saved"))?,
                )
                .map_err(db)?;
        }
        Ok(SourceAccount { user, sources })
    }

    /// Reject requests from an older account snapshot before changing any source selection.
    pub fn select_source(&mut self, expected_user: i64, source_id: i64) -> io::Result<Source> {
        let encoded = lw_apple_sys::cloud::current_identity()?;
        let user = self.user_for_identity(encoded.as_deref())?;
        self.select_source_for_user(&user, expected_user, source_id)
    }

    fn select_source_for_user(
        &mut self,
        user: &User,
        expected_user: i64,
        source_id: i64,
    ) -> io::Result<Source> {
        if user.id != expected_user {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "The iCloud account changed. Reload folders.",
            ));
        }
        self.repository
            .set_current_source(user.id, source_id)
            .map_err(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    #[test]
    fn source_accounts_preserve_swift_identity_current_selection_and_other_account_folders() {
        let base = std::env::temp_dir().join(format!("lw-source-account-{}", std::process::id()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        std::fs::create_dir_all(&base).unwrap();
        let _cleanup = Cleanup(base.clone());
        let mut worker = LibraryWorker::open(
            &base.join("library.sqlite"),
            Preferences::open(Some(&format!(
                "localwave.source-account.{}",
                std::process::id()
            )))
            .unwrap(),
            AppPaths {
                documents: base.join("Documents"),
                caches: base.join("Caches"),
                temporary: base.join("tmp"),
                test_database: None,
            },
        )
        .unwrap();
        let mut fixture = include_str!("../../../lw_apple_sys/fixtures/cloud-identity.txt").lines();
        let encoded = fixture.next().unwrap();
        let swift_id = fixture.next().unwrap().parse::<i64>().unwrap();
        let existing = worker.repository.get_or_create_user(swift_id).unwrap();
        let cloud = worker.user_for_identity(Some(encoded)).unwrap();
        assert_eq!(
            cloud, existing,
            "the Swift account must reopen without creating a new owner"
        );
        let other = worker
            .user_for_identity(Some("another-opaque-account"))
            .unwrap();
        let local = worker.user_for_identity(None).unwrap();
        assert_eq!(local.icloud_id, -1);
        assert!(
            worker
                .sources_for_user(cloud.clone())
                .unwrap()
                .sources
                .is_empty()
        );
        let create = |worker: &LibraryWorker, user: &User, path: &str| {
            worker
                .repository
                .create_source(&Source {
                    user_id: user.id,
                    dir_path: path.into(),
                    path_id: hash_string(path),
                    created_at: 1.0,
                    ..Default::default()
                })
                .unwrap()
        };
        let first = create(&worker, &cloud, "/first");
        let second = create(&worker, &cloud, "/second");
        let foreign = create(&worker, &other, "/other");
        let local_source = create(&worker, &local, "/local");
        let listing = worker.sources_for_user(cloud.clone()).unwrap();
        assert_eq!(listing.user, cloud);
        assert_eq!(
            listing
                .sources
                .iter()
                .map(|source| source.id)
                .collect::<Vec<_>>(),
            vec![first.id, second.id]
        );
        assert!(listing.sources[0].is_current);
        worker
            .select_source_for_user(&cloud, cloud.id, second.id.unwrap())
            .unwrap();
        assert!(
            worker
                .select_source_for_user(&cloud, cloud.id, foreign.id.unwrap())
                .is_err()
        );
        assert_eq!(
            worker
                .select_source_for_user(&other, cloud.id, foreign.id.unwrap())
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(
            worker
                .repository
                .source(second.id.unwrap())
                .unwrap()
                .unwrap()
                .is_current
        );
        assert!(
            !worker
                .repository
                .source(foreign.id.unwrap())
                .unwrap()
                .unwrap()
                .is_current
        );
        let listing = worker.sources_for_user(local).unwrap();
        assert_eq!(listing.sources.len(), 1);
        assert_eq!(listing.sources[0].id, local_source.id);
        let restored = worker.user_for_identity(Some(encoded)).unwrap();
        let listing = worker.sources_for_user(restored).unwrap();
        assert_eq!(
            listing
                .sources
                .iter()
                .find(|source| source.is_current)
                .unwrap()
                .id,
            second.id
        );
        assert!(
            worker
                .repository
                .source(foreign.id.unwrap())
                .unwrap()
                .is_some()
        );
    }
}
