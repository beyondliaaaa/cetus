//! Test helpers shared by the workspace: sample models, compile helpers,
//! trajectory dumps and cross-target hash fixtures.

#![forbid(unsafe_code)]

/// Returns the path of the workspace root, for tests that read fixtures.
///
/// # Panics
/// Panics if the crate is built outside the workspace (no `CARGO_MANIFEST_DIR`
/// ancestor named `crates`). This is a test-helper crate, so that is fine.
#[must_use]
pub fn workspace_root() -> std::path::PathBuf {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(dir.pop(), "manifest dir has no parent");
    assert!(dir.pop(), "workspace root missing");
    dir
}
