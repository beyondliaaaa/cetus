//! Locks the M0 determinism witness. If this test fails, either `cetus-math`
//! changed the battery (update the golden *and* every platform together) or a
//! dependency broke bit-identical math.
//!
//! The golden lives in `golden/hashes.txt`, shared with
//! `scripts/check-cross-target.sh`, which checks the wasm build against it.

const GOLDEN: &str = include_str!("../golden/hashes.txt");

fn golden(key: &str) -> &'static str {
    GOLDEN
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find_map(|line| {
            let (k, v) = line.split_once(char::is_whitespace)?;
            (k == key).then(|| v.trim())
        })
        .unwrap_or_else(|| panic!("golden/hashes.txt has no entry for `{key}`"))
}

#[test]
fn hello_digest_matches_golden() {
    let hex = cetus_kernel::hello_digest_hex();
    assert_eq!(
        hex,
        golden("hello"),
        "hello digest drifted; see the header of golden/hashes.txt before updating it"
    );
}
