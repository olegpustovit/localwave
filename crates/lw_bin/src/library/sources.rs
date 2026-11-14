//! Source ownership follows the same secure iCloud identity as the shipping Swift app.
use super::{LibraryWorker, Root, check, db, hash, text};
use lw_apple_sys::{
    bookmarks::{self, Access},
    preferences::file_path,
    scan::Scan,
};
use lw_core::{Source, User, hash_string};
use lw_ui::files::Target;
use std::{io, path::Path, sync::atomic::AtomicBool};

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
        let user = self.current_source_user(expected_user)?;
        self.select_source_for_user(&user, expected_user, source_id)
    }

    pub(super) fn current_source_user(&mut self, expected_user: i64) -> io::Result<User> {
        let encoded = lw_apple_sys::cloud::current_identity()?;
        let user = self.user_for_identity(encoded.as_deref())?;
        require_user(&user, expected_user)?;
        Ok(user)
    }

    pub fn owned_source(&mut self, expected_user: i64, source_id: i64) -> io::Result<Source> {
        self.current_source_user(expected_user)?;
        let source = self
            .repository
            .source(source_id)
            .map_err(db)?
            .ok_or_else(|| io::Error::other("This music source was removed"))?;
        if source.user_id != expected_user {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "This source belongs to another account",
            ));
        }
        Ok(source)
    }

    /// Validate access and generated-path exclusions before committing a bookmark or selection.
    pub fn register_folder(
        &mut self,
        expected_user: i64,
        folder: &Path,
        now: f64,
        cancelled: &AtomicBool,
    ) -> io::Result<Source> {
        let user = self.current_source_user(expected_user)?;
        self.register_folder_for_user(&user, expected_user, folder, now, cancelled)
    }

    fn source_target(&mut self, target: Target) -> io::Result<Source> {
        let source = self.owned_source(target.user, target.source)?;
        if source.path_id != target.root {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "This folder changed. Reload folders and try again.",
            ));
        }
        Ok(source)
    }

    pub fn reconnect_folder(
        &mut self,
        target: Target,
        folder: &Path,
        now: f64,
        cancelled: &AtomicBool,
    ) -> io::Result<Source> {
        check(cancelled)?;
        let expected = self.source_target(target)?;
        let (bookmark, access) = self.folder_access(folder, now, cancelled)?;
        let locations = self.source_locations(&expected, access.path())?;
        let mut resolved = expected.clone();
        resolved.dir_path = text(folder)?.to_owned();
        resolved.path_id = hash(folder)?;
        resolved.bookmark = Some(bookmark.clone());
        resolved.sync_error = None;
        resolved.updated_at = Some(now);
        self.current_source_user(target.user)?;
        check(cancelled)?;
        let saved = self
            .repository
            .save_source_access(&expected, &resolved, &locations, now)
            .map_err(db)?;
        self.preferences
            .set_bookmark(saved.path_id, Some(&bookmark))?;
        self.roots.insert(target.source, Root { bookmark, access });
        Ok(saved)
    }

    pub fn remove_source(&mut self, target: Target, cancelled: &AtomicBool) -> io::Result<()> {
        check(cancelled)?;
        let expected = self.source_target(target)?;
        let registered = file_path(&expected.dir_path)?;
        let root = self
            .roots
            .get(&target.source)
            .map_or(registered.as_path(), |root| root.access.path());
        let locations = self.source_locations(&expected, root)?;
        self.current_source_user(target.user)?;
        check(cancelled)?;
        self.repository
            .delete_source_with_locations(&expected, &locations)
            .map_err(db)?;
        self.roots.remove(&target.source);
        Ok(())
    }

    pub(super) fn source_locations(
        &self,
        source: &Source,
        root: &Path,
    ) -> io::Result<Vec<(i64, String)>> {
        let id = source
            .id
            .ok_or_else(|| io::Error::other("Source is not saved"))?;
        let registered = file_path(&source.dir_path)?;
        let canonical = registered
            .canonicalize()
            .unwrap_or_else(|_| registered.clone());
        let mut locations = Vec::new();
        for path in self
            .repository
            .source_paths(id, None)
            .map_err(db)?
            .into_iter()
            .filter(|path| !path.is_directory)
        {
            if !lw_core::models::valid_relative_path(&path.relative_path) {
                return Err(io::Error::other("Invalid legacy source-relative path"));
            }
            for identity in [
                path.path_id,
                hash(&registered.join(&path.relative_path))?,
                hash(&canonical.join(&path.relative_path))?,
                hash(&root.join(&path.relative_path))?,
            ] {
                locations.push((identity, path.relative_path.clone()));
            }
        }
        Ok(locations)
    }

    fn folder_access(
        &self,
        folder: &Path,
        now: f64,
        cancelled: &AtomicBool,
    ) -> io::Result<(Vec<u8>, Access)> {
        check(cancelled)?;
        if !now.is_finite() || !folder.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Expected an absolute folder and finite date",
            ));
        }
        let bookmark = bookmarks::create(folder)?;
        if bookmark.len() > 64 * 1024 * 1024 {
            return Err(io::Error::other("Bookmark exceeds its memory limit"));
        }
        let access = Access::resolve(&bookmark)?;
        drop(Scan::open(&access, folder, &self.excluded)?);
        check(cancelled)?;
        Ok((bookmark, access))
    }

    fn register_folder_for_user(
        &mut self,
        user: &User,
        expected_user: i64,
        folder: &Path,
        now: f64,
        cancelled: &AtomicBool,
    ) -> io::Result<Source> {
        require_user(user, expected_user)?;
        let (bookmark, access) = self.folder_access(folder, now, cancelled)?;
        let url = lw_apple_sys::normalized_file_url(text(folder)?)?;
        let key = hash_string(&url);
        // Keep a legacy file-URL spelling so reselecting the same Swift folder reuses its row.
        let existing = self
            .repository
            .sources(user.id)
            .map_err(db)?
            .into_iter()
            .find(|source| {
                source.path_id == key
                    && file_path(&source.dir_path).is_ok_and(|path| {
                        text(&path)
                            .ok()
                            .and_then(|path| lw_apple_sys::normalized_file_url(path).ok())
                            .as_deref()
                            == Some(url.as_str())
                    })
            });
        let directory = existing
            .as_ref()
            .map_or_else(|| text(folder), |source| Ok(source.dir_path.as_str()))?;
        check(cancelled)?;
        let source = self
            .repository
            .register_source_folder(user.id, directory, key, &bookmark, now)
            .map_err(db)?;
        self.preferences.set_bookmark(key, Some(&bookmark))?;
        self.roots.insert(
            source
                .id
                .ok_or_else(|| io::Error::other("Source is not saved"))?,
            Root { bookmark, access },
        );
        Ok(source)
    }

    fn select_source_for_user(
        &mut self,
        user: &User,
        expected_user: i64,
        source_id: i64,
    ) -> io::Result<Source> {
        require_user(user, expected_user)?;
        self.repository
            .set_current_source(user.id, source_id)
            .map_err(db)
    }
}

fn require_user(user: &User, expected: i64) -> io::Result<()> {
    if user.id == expected {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "The iCloud account changed. Reload folders.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lw_apple_sys::preferences::{AppPaths, Preferences};
    #[test]
    fn folder_registration_validates_grants_exclusions_cancellation_and_legacy_identity() {
        let base = std::env::temp_dir().join(format!("lw-register-folder-{}", std::process::id()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let original = base.join("Música 名前");
        let documents = base.join("Documents");
        std::fs::create_dir_all(&original).unwrap();
        std::fs::create_dir_all(documents.join("CoverArt")).unwrap();
        std::fs::write(
            original.join("audio.flac"),
            include_bytes!("../../../../Tests/LocalWaveDataTests/Fixtures/tagged.flac"),
        )
        .unwrap();
        let mut worker = LibraryWorker::open(
            &base.join("library.sqlite"),
            Preferences::open(Some(&format!(
                "localwave.register-folder.{}",
                std::process::id()
            )))
            .unwrap(),
            AppPaths {
                documents: documents.clone(),
                caches: base.join("Caches"),
                temporary: base.join("tmp"),
                test_database: None,
            },
        )
        .unwrap();
        let user = worker.user_for_identity(None).unwrap();
        let other = worker.user_for_identity(Some("other")).unwrap();
        let url = lw_apple_sys::normalized_file_url(original.to_str().unwrap()).unwrap();
        let key = hash_string(&url);
        let legacy = worker
            .repository
            .create_source(&Source {
                user_id: user.id,
                dir_path: url.clone(),
                path_id: key,
                created_at: 1.0,
                ..Source::default()
            })
            .unwrap();
        let cancel = AtomicBool::new(false);
        let registered = worker
            .register_folder_for_user(&user, user.id, &original, 2.0, &cancel)
            .unwrap();
        assert_eq!(registered.id, legacy.id);
        assert_eq!(registered.dir_path, url);
        assert!(registered.is_current);
        assert!(
            registered
                .bookmark
                .as_ref()
                .is_some_and(|bookmark| !bookmark.is_empty())
        );
        assert_eq!(
            worker.preferences.bookmark(key).unwrap(),
            registered.bookmark
        );
        assert_eq!(worker.repository.sources(user.id).unwrap().len(), 1);
        assert_eq!(worker.roots.len(), 1);
        let saved = registered.clone();
        for invalid in [
            documents.join("CoverArt"),
            original.join("audio.flac"),
            base.join("missing"),
            std::path::PathBuf::from("relative"),
        ] {
            assert!(
                worker
                    .register_folder_for_user(&user, user.id, &invalid, 3.0, &cancel)
                    .is_err()
            );
        }
        assert!(
            worker
                .register_folder_for_user(&user, user.id, &original, f64::NAN, &cancel)
                .is_err()
        );
        assert_eq!(
            worker
                .register_folder_for_user(&other, user.id, &original, 3.0, &cancel)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            worker
                .register_folder_for_user(&user, user.id, &original, 3.0, &cancel)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(worker.repository.sources(user.id).unwrap(), [saved]);
        assert!(worker.repository.sources(other.id).unwrap().is_empty());
    }

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
