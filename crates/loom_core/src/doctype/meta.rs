use serde::{Deserialize, Serialize};

/// Complete metadata for a DocType — the central data model definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    pub module: String,

    #[serde(default)]
    pub is_submittable: bool,
    #[serde(default)]
    pub is_child_table: bool,
    #[serde(default)]
    pub is_single: bool,
    #[serde(default)]
    pub is_virtual: bool,
    #[serde(default)]
    pub is_tree: bool,

    #[serde(default)]
    pub naming_rule: NamingRule,
    #[serde(default)]
    pub autoname: Option<String>,

    #[serde(default)]
    pub title_field: Option<String>,
    #[serde(default)]
    pub search_fields: Vec<String>,
    #[serde(default)]
    pub sort_field: Option<String>,
    #[serde(default)]
    pub sort_order: Option<String>,

    #[serde(default)]
    pub fields: Vec<DocFieldMeta>,
    #[serde(default)]
    pub permissions: Vec<DocPermMeta>,

    /// Optional workflow state machine for submittable documents.
    #[serde(default)]
    pub workflow: Option<super::workflow::Workflow>,
}

/// Metadata for a single field within a DocType.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocFieldMeta {
    pub fieldname: String,
    #[serde(default)]
    pub label: Option<String>,
    pub fieldtype: FieldType,

    /// For Link fields: the target DocType name.
    /// For Select fields: newline-separated options.
    /// For Table fields: the child DocType name.
    #[serde(default)]
    pub options: Option<String>,

    #[serde(default)]
    pub reqd: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub set_only_once: bool,

    #[serde(default)]
    pub default: Option<serde_json::Value>,

    #[serde(default)]
    pub in_list_view: bool,
    #[serde(default)]
    pub in_standard_filter: bool,

    /// e.g. "employee.employee_name" — auto-fetch from linked doc
    #[serde(default)]
    pub fetch_from: Option<String>,

    /// JS/Rhai expression controlling field visibility
    #[serde(default)]
    pub depends_on: Option<String>,
    #[serde(default)]
    pub mandatory_depends_on: Option<String>,
    #[serde(default)]
    pub read_only_depends_on: Option<String>,

    /// For collapsible sections
    #[serde(default)]
    pub collapsible: bool,

    #[serde(default)]
    pub description: Option<String>,

    /// Permission level (0-9). Fields with level > 0 require a matching perm rule.
    #[serde(default)]
    pub permlevel: u8,

    /// Length for Data/varchar fields
    #[serde(default)]
    pub length: Option<u32>,

    /// Precision for Float/Currency fields
    #[serde(default)]
    pub precision: Option<u8>,
}

/// Permission rule for a DocType.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPermMeta {
    pub role: String,
    #[serde(default)]
    pub permlevel: u8,
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub write: bool,
    #[serde(default)]
    pub create: bool,
    #[serde(default)]
    pub delete: bool,
    #[serde(default)]
    pub submit: bool,
    #[serde(default)]
    pub cancel: bool,
    #[serde(default)]
    pub amend: bool,
    #[serde(default)]
    pub report: bool,
    #[serde(default)]
    pub export: bool,
    #[serde(default)]
    pub print: bool,
    #[serde(default)]
    pub email: bool,
    #[serde(default)]
    pub share: bool,
    #[serde(default)]
    pub if_owner: bool,
}

/// All supported field types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    Data,
    Link,
    DynamicLink,
    Select,
    Int,
    Float,
    Currency,
    Percent,
    Check,
    Date,
    Datetime,
    Time,
    Text,
    SmallText,
    LongText,
    Code,
    TextEditor,
    HTMLEditor,
    Password,
    Attach,
    AttachImage,
    Table,
    Color,
    Geolocation,
    JSON,
    // Layout fields (no data storage)
    SectionBreak,
    ColumnBreak,
    TabBreak,
}

impl FieldType {
    /// Returns true if this field type stores data in the database.
    pub fn has_column(&self) -> bool {
        !matches!(
            self,
            FieldType::SectionBreak | FieldType::ColumnBreak | FieldType::TabBreak
        )
    }

    /// Returns the SQL column type for this field type (PostgreSQL).
    pub fn sql_type(&self) -> &'static str {
        match self {
            FieldType::Data
            | FieldType::Link
            | FieldType::DynamicLink
            | FieldType::Select
            | FieldType::Password
            | FieldType::Color => "VARCHAR(140)",
            FieldType::Int => "BIGINT",
            FieldType::Float | FieldType::Percent => "DOUBLE PRECISION",
            FieldType::Currency => "NUMERIC(18, 6)",
            FieldType::Check => "BOOLEAN",
            FieldType::Date => "DATE",
            FieldType::Datetime => "TIMESTAMP",
            FieldType::Time => "TIME",
            FieldType::Text
            | FieldType::SmallText
            | FieldType::Code
            | FieldType::TextEditor
            | FieldType::HTMLEditor => "TEXT",
            FieldType::LongText => "TEXT",
            FieldType::Attach | FieldType::AttachImage => "TEXT",
            FieldType::Table => "TEXT", // stored as JSON array
            FieldType::Geolocation | FieldType::JSON => "JSONB",
            // Layout fields — no column
            FieldType::SectionBreak | FieldType::ColumnBreak | FieldType::TabBreak => "",
        }
    }
}

/// Rules for generating the `id` (primary key) of a document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamingRule {
    #[default]
    Autoincrement,
    Hash,
    #[serde(rename = "by_fieldname")]
    ByFieldname,
    Series,
    Prompt,
    Expression,
}

/// Standard fields that are automatically added to every DocType table.
pub const STANDARD_FIELDS: &[(&str, &str)] = &[
    ("id", "VARCHAR(140) PRIMARY KEY"),
    ("owner", "VARCHAR(140)"),
    ("creation", "TIMESTAMP DEFAULT NOW()"),
    ("modified", "TIMESTAMP DEFAULT NOW()"),
    ("modified_by", "VARCHAR(140)"),
    ("docstatus", "SMALLINT DEFAULT 0"),
    ("idx", "INTEGER DEFAULT 0"),
    ("parent", "VARCHAR(140)"),
    ("parentfield", "VARCHAR(140)"),
    ("parenttype", "VARCHAR(140)"),
];

impl Meta {
    /// Create the bootstrapped "DocType" DocType — the meta-circular definition
    /// that allows creating new DocTypes through the API.
    pub fn doctype_meta() -> Self {
        Self {
            name: "DocType".to_string(),
            module: "Core".to_string(),
            naming_rule: NamingRule::Prompt,
            fields: vec![
                DocFieldMeta {
                    fieldname: "module".to_string(),
                    label: Some("Module".to_string()),
                    fieldtype: FieldType::Data,
                    in_list_view: true,
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "is_submittable".to_string(),
                    label: Some("Is Submittable".to_string()),
                    fieldtype: FieldType::Check,
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "is_child_table".to_string(),
                    label: Some("Is Child Table".to_string()),
                    fieldtype: FieldType::Check,
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "is_single".to_string(),
                    label: Some("Is Single".to_string()),
                    fieldtype: FieldType::Check,
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "naming_rule".to_string(),
                    label: Some("Naming Rule".to_string()),
                    fieldtype: FieldType::Select,
                    options: Some(
                        "autoincrement\nhash\nby_fieldname\nseries\nprompt\nexpression".to_string(),
                    ),
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "autoname".to_string(),
                    label: Some("Auto Name".to_string()),
                    fieldtype: FieldType::Data,
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "title_field".to_string(),
                    label: Some("Title Field".to_string()),
                    fieldtype: FieldType::Data,
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "sort_field".to_string(),
                    label: Some("Sort Field".to_string()),
                    fieldtype: FieldType::Data,
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "sort_order".to_string(),
                    label: Some("Sort Order".to_string()),
                    fieldtype: FieldType::Select,
                    options: Some("asc\ndesc".to_string()),
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "fields_json".to_string(),
                    label: Some("Fields".to_string()),
                    fieldtype: FieldType::JSON,
                    ..DocFieldMeta::default()
                },
                DocFieldMeta {
                    fieldname: "permissions_json".to_string(),
                    label: Some("Permissions".to_string()),
                    fieldtype: FieldType::JSON,
                    ..DocFieldMeta::default()
                },
            ],
            permissions: vec![
                DocPermMeta {
                    role: "Administrator".to_string(),
                    read: true,
                    write: true,
                    create: true,
                    delete: true,
                    ..DocPermMeta::default()
                },
                DocPermMeta {
                    role: "System Manager".to_string(),
                    read: true,
                    write: true,
                    create: true,
                    delete: true,
                    ..DocPermMeta::default()
                },
            ],
            ..Self::default()
        }
    }

    /// Load a DocType definition from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Load a DocType definition from a JSON file.
    pub fn from_json_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_json(&content)?)
    }

    /// Get the database table name for this DocType.
    /// Convention: lowercase, spaces replaced with underscores, prefixed with `tab`.
    pub fn table_name(&self) -> String {
        doctype_table_name(&self.name)
    }

    /// Get all fields that have database columns (excludes layout fields).
    pub fn data_fields(&self) -> impl Iterator<Item = &DocFieldMeta> {
        self.fields.iter().filter(|f| f.fieldtype.has_column())
    }

    /// Get a field by fieldname.
    pub fn get_field(&self, fieldname: &str) -> Option<&DocFieldMeta> {
        self.fields.iter().find(|f| f.fieldname == fieldname)
    }

    /// Get all required fields.
    pub fn required_fields(&self) -> impl Iterator<Item = &DocFieldMeta> {
        self.fields.iter().filter(|f| f.reqd)
    }

    /// Get all Link fields.
    pub fn link_fields(&self) -> impl Iterator<Item = &DocFieldMeta> {
        self.fields
            .iter()
            .filter(|f| f.fieldtype == FieldType::Link)
    }
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            name: String::new(),
            module: String::new(),
            is_submittable: false,
            is_child_table: false,
            is_single: false,
            is_virtual: false,
            is_tree: false,
            naming_rule: NamingRule::default(),
            autoname: None,
            title_field: None,
            search_fields: Vec::new(),
            sort_field: None,
            sort_order: None,
            fields: Vec::new(),
            permissions: Vec::new(),
            workflow: None,
        }
    }
}

impl Default for DocFieldMeta {
    fn default() -> Self {
        Self {
            fieldname: String::new(),
            label: None,
            fieldtype: FieldType::Data,
            options: None,
            reqd: false,
            unique: false,
            read_only: false,
            hidden: false,
            set_only_once: false,
            default: None,
            in_list_view: false,
            in_standard_filter: false,
            fetch_from: None,
            depends_on: None,
            mandatory_depends_on: None,
            read_only_depends_on: None,
            collapsible: false,
            description: None,
            permlevel: 0,
            length: None,
            precision: None,
        }
    }
}

impl Default for DocPermMeta {
    fn default() -> Self {
        Self {
            role: String::new(),
            permlevel: 0,
            read: false,
            write: false,
            create: false,
            delete: false,
            submit: false,
            cancel: false,
            amend: false,
            report: false,
            export: false,
            print: false,
            email: false,
            share: false,
            if_owner: false,
        }
    }
}

// =========================================================================
// Shared utility functions
// =========================================================================

/// Get the database table name for a DocType name string.
/// Convention: lowercase, snake_case.
/// e.g., "Leave Application" → "leave_application", "User" → "user"
pub fn doctype_table_name(doctype: &str) -> String {
    let mut result = String::new();
    for (i, ch) in doctype.chars().enumerate() {
        if ch == ' ' || ch == '-' {
            result.push('_');
        } else if ch.is_uppercase() {
            // Insert underscore before uppercase letters (camelCase → snake_case)
            // but not at the start or after a space/underscore
            if i > 0 {
                let prev = doctype.chars().nth(i - 1).unwrap_or('_');
                if prev != ' ' && prev != '_' && prev != '-' && !prev.is_uppercase() {
                    result.push('_');
                }
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

/// Merge permission overrides on top of defaults.
/// Matching `(role, permlevel)` pairs are replaced; new override rules are appended.
/// Empty overrides returns defaults unchanged.
pub fn merge_permission_overrides(
    defaults: &[DocPermMeta],
    overrides: &[DocPermMeta],
) -> Vec<DocPermMeta> {
    if overrides.is_empty() {
        return defaults.to_vec();
    }

    let override_keys: std::collections::HashSet<(String, u8)> = overrides
        .iter()
        .map(|p| (p.role.clone(), p.permlevel))
        .collect();
    let mut default_keys = std::collections::HashSet::new();
    let mut merged = Vec::new();

    for d in defaults {
        let key = (d.role.clone(), d.permlevel);
        default_keys.insert(key.clone());
        if override_keys.contains(&key) {
            if let Some(o) = overrides
                .iter()
                .find(|o| o.role == d.role && o.permlevel == d.permlevel)
            {
                merged.push(o.clone());
            }
        } else {
            merged.push(d.clone());
        }
    }

    for o in overrides {
        let key = (o.role.clone(), o.permlevel);
        if !default_keys.contains(&key) {
            merged.push(o.clone());
        }
    }

    merged
}

/// Set standard fields for a new document (insert).
pub fn set_standard_fields_on_insert(doc: &mut serde_json::Value, user: &str) {
    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    if let Some(obj) = doc.as_object_mut() {
        obj.entry("owner")
            .or_insert_with(|| serde_json::json!(user));
        obj.entry("creation")
            .or_insert_with(|| serde_json::json!(&now));
        obj.insert("modified".to_string(), serde_json::json!(&now));
        obj.insert("modified_by".to_string(), serde_json::json!(user));
        obj.entry("docstatus")
            .or_insert_with(|| serde_json::json!(0));
    }
}

/// Set standard fields for an existing document (update).
pub fn set_standard_fields_on_update(doc: &mut serde_json::Value, user: &str) {
    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("modified".to_string(), serde_json::json!(&now));
        obj.insert("modified_by".to_string(), serde_json::json!(user));
    }
}
