use loom_core::doctype::meta::DocPermMeta;
use loom_core::perms::docperm::{perm_grants, PermType};

// ===========================================================================
// Helper: build a DocPermMeta with all permissions enabled
// ===========================================================================

fn all_perms() -> DocPermMeta {
    DocPermMeta {
        role: "Test Role".to_string(),
        permlevel: 0,
        read: true,
        write: true,
        create: true,
        delete: true,
        submit: true,
        cancel: true,
        amend: true,
        report: true,
        export: true,
        print: true,
        email: true,
        share: true,
        if_owner: false,
    }
}

// ===========================================================================
// perm_grants tests — one per PermType variant
// ===========================================================================

#[test]
fn test_perm_grants_read() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Read));

    let mut no_read = all_perms();
    no_read.read = false;
    assert!(!perm_grants(&no_read, PermType::Read));
}

#[test]
fn test_perm_grants_write() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Write));

    let mut no = all_perms();
    no.write = false;
    assert!(!perm_grants(&no, PermType::Write));
}

#[test]
fn test_perm_grants_create() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Create));

    let mut no = all_perms();
    no.create = false;
    assert!(!perm_grants(&no, PermType::Create));
}

#[test]
fn test_perm_grants_delete() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Delete));

    let mut no = all_perms();
    no.delete = false;
    assert!(!perm_grants(&no, PermType::Delete));
}

#[test]
fn test_perm_grants_submit() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Submit));

    let mut no = all_perms();
    no.submit = false;
    assert!(!perm_grants(&no, PermType::Submit));
}

#[test]
fn test_perm_grants_cancel() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Cancel));

    let mut no = all_perms();
    no.cancel = false;
    assert!(!perm_grants(&no, PermType::Cancel));
}

#[test]
fn test_perm_grants_amend() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Amend));

    let mut no = all_perms();
    no.amend = false;
    assert!(!perm_grants(&no, PermType::Amend));
}

#[test]
fn test_perm_grants_report() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Report));

    let mut no = all_perms();
    no.report = false;
    assert!(!perm_grants(&no, PermType::Report));
}

#[test]
fn test_perm_grants_export() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Export));

    let mut no = all_perms();
    no.export = false;
    assert!(!perm_grants(&no, PermType::Export));
}

#[test]
fn test_perm_grants_print() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Print));

    let mut no = all_perms();
    no.print = false;
    assert!(!perm_grants(&no, PermType::Print));
}

#[test]
fn test_perm_grants_email() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Email));

    let mut no = all_perms();
    no.email = false;
    assert!(!perm_grants(&no, PermType::Email));
}

#[test]
fn test_perm_grants_share() {
    let perm = all_perms();
    assert!(perm_grants(&perm, PermType::Share));

    let mut no = all_perms();
    no.share = false;
    assert!(!perm_grants(&no, PermType::Share));
}

#[test]
fn test_perm_grants_false_by_default() {
    let perm = DocPermMeta::default();
    assert!(!perm_grants(&perm, PermType::Read));
    assert!(!perm_grants(&perm, PermType::Write));
    assert!(!perm_grants(&perm, PermType::Create));
    assert!(!perm_grants(&perm, PermType::Delete));
    assert!(!perm_grants(&perm, PermType::Submit));
    assert!(!perm_grants(&perm, PermType::Cancel));
    assert!(!perm_grants(&perm, PermType::Amend));
    assert!(!perm_grants(&perm, PermType::Report));
    assert!(!perm_grants(&perm, PermType::Export));
    assert!(!perm_grants(&perm, PermType::Print));
    assert!(!perm_grants(&perm, PermType::Email));
    assert!(!perm_grants(&perm, PermType::Share));
}

// ===========================================================================
// PermType::as_str tests
// ===========================================================================

#[test]
fn test_perm_type_as_str() {
    assert_eq!(PermType::Read.as_str(), "read");
    assert_eq!(PermType::Write.as_str(), "write");
    assert_eq!(PermType::Create.as_str(), "create");
    assert_eq!(PermType::Delete.as_str(), "delete");
    assert_eq!(PermType::Submit.as_str(), "submit");
    assert_eq!(PermType::Cancel.as_str(), "cancel");
    assert_eq!(PermType::Amend.as_str(), "amend");
    assert_eq!(PermType::Report.as_str(), "report");
    assert_eq!(PermType::Export.as_str(), "export");
    assert_eq!(PermType::Print.as_str(), "print");
    assert_eq!(PermType::Email.as_str(), "email");
    assert_eq!(PermType::Share.as_str(), "share");
}
