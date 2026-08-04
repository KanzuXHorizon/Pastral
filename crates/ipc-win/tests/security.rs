use pastral_ipc_win::{
    build_logon_sid_pipe_security, current_token_identity, inspect_pipe_security,
};

const GENERIC_READ_WRITE_SYNCHRONIZE: u32 = 0xc010_0000;

#[test]
fn descriptor_is_protected_logon_sid_only_and_not_defaulted() {
    let identity = current_token_identity().unwrap();
    let security = build_logon_sid_pipe_security(&identity).unwrap();
    let inspection = inspect_pipe_security(&security).unwrap();

    assert!(inspection.dacl_present());
    assert!(!inspection.dacl_defaulted());
    assert!(inspection.dacl_protected());
    assert_eq!(inspection.ace_count(), 1);
    assert_eq!(inspection.allow_ace_count(), 1);
    assert!(inspection.exact_logon_sid_match());
    assert_eq!(inspection.access_mask(), GENERIC_READ_WRITE_SYNCHRONIZE);
}
