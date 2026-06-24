//! Enforcement backend mapping tests.

#[test]
fn signed_backend_defaults() {
    let m =
        shared_types::EnforcementMapping::from_backend(shared_types::EnforcementBackend::Signed);
    assert_eq!(m.guardian_mode, shared_types::GuardianMode::Wfp);
    assert_eq!(m.wfp_engine_impl, "userspace");
    assert!(m.use_windivert);
}

#[test]
fn custom_kernel_backend_maps_hybrid() {
    let m = shared_types::EnforcementMapping::from_backend(
        shared_types::EnforcementBackend::CustomKernel,
    );
    assert_eq!(m.guardian_mode, shared_types::GuardianMode::Hybrid);
    assert_eq!(m.wfp_engine_impl, "kernel");
    assert!(!m.use_windivert);
}

#[test]
fn enforcement_backend_parse_roundtrip() {
    assert_eq!(
        shared_types::EnforcementBackend::parse("signed"),
        shared_types::EnforcementBackend::Signed
    );
    assert_eq!(
        shared_types::EnforcementBackend::parse("custom_kernel"),
        shared_types::EnforcementBackend::CustomKernel
    );
}
