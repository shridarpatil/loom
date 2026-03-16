use crate::doctype::meta::DocPermMeta;

/// Permission types that can be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermType {
    Read,
    Write,
    Create,
    Delete,
    Submit,
    Cancel,
    Amend,
    Report,
    Export,
    Print,
    Email,
    Share,
}

impl PermType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermType::Read => "read",
            PermType::Write => "write",
            PermType::Create => "create",
            PermType::Delete => "delete",
            PermType::Submit => "submit",
            PermType::Cancel => "cancel",
            PermType::Amend => "amend",
            PermType::Report => "report",
            PermType::Export => "export",
            PermType::Print => "print",
            PermType::Email => "email",
            PermType::Share => "share",
        }
    }
}

/// Check if a permission rule grants the given permission type.
pub fn perm_grants(perm: &DocPermMeta, ptype: PermType) -> bool {
    match ptype {
        PermType::Read => perm.read,
        PermType::Write => perm.write,
        PermType::Create => perm.create,
        PermType::Delete => perm.delete,
        PermType::Submit => perm.submit,
        PermType::Cancel => perm.cancel,
        PermType::Amend => perm.amend,
        PermType::Report => perm.report,
        PermType::Export => perm.export,
        PermType::Print => perm.print,
        PermType::Email => perm.email,
        PermType::Share => perm.share,
    }
}
