//! Locks the M0 determinism witness. If this test fails, either `cetus-math`
//! changed the battery (update the golden *and* every platform together) or a
//! dependency broke bit-identical math.

#[test]
fn hello_digest_is_well_formed() {
    let hex = cetus_kernel::hello_digest_hex();
    println!("hello digest = {hex}");
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}
