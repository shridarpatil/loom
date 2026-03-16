use serde_json::Value;

/// A fluent query builder for constructing SELECT queries against DocType tables.
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    pub doctype: String,
    pub table: String,
    pub select_fields: Vec<String>,
    pub filters: Vec<Filter>,
    pub or_filters: Vec<Vec<Filter>>,
    pub order_by: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub group_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOp,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    Like,
    NotLike,
    In,
    NotIn,
    IsNull,
    IsNotNull,
    Between,
}

impl FilterOp {
    pub fn as_sql(&self) -> &'static str {
        match self {
            FilterOp::Eq => "=",
            FilterOp::Ne => "!=",
            FilterOp::Gt => ">",
            FilterOp::Lt => "<",
            FilterOp::Gte => ">=",
            FilterOp::Lte => "<=",
            FilterOp::Like => "LIKE",
            FilterOp::NotLike => "NOT LIKE",
            FilterOp::In => "IN",
            FilterOp::NotIn => "NOT IN",
            FilterOp::IsNull => "IS NULL",
            FilterOp::IsNotNull => "IS NOT NULL",
            FilterOp::Between => "BETWEEN",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "=" => Some(FilterOp::Eq),
            "!=" | "<>" => Some(FilterOp::Ne),
            ">" => Some(FilterOp::Gt),
            "<" => Some(FilterOp::Lt),
            ">=" => Some(FilterOp::Gte),
            "<=" => Some(FilterOp::Lte),
            "like" | "LIKE" => Some(FilterOp::Like),
            "not like" | "NOT LIKE" => Some(FilterOp::NotLike),
            "in" | "IN" => Some(FilterOp::In),
            "not in" | "NOT IN" => Some(FilterOp::NotIn),
            "is" | "IS" => Some(FilterOp::IsNull),
            "is not" | "IS NOT" => Some(FilterOp::IsNotNull),
            "between" | "BETWEEN" => Some(FilterOp::Between),
            _ => None,
        }
    }
}

impl QueryBuilder {
    pub fn new(doctype: &str) -> Self {
        let table = crate::doctype::meta::doctype_table_name(doctype);
        Self {
            doctype: doctype.to_string(),
            table,
            select_fields: vec!["*".to_string()],
            filters: Vec::new(),
            or_filters: Vec::new(),
            order_by: None,
            limit: None,
            offset: None,
            group_by: None,
        }
    }

    pub fn fields(mut self, fields: &[&str]) -> Self {
        self.select_fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn filter(mut self, field: &str, op: FilterOp, value: Value) -> Self {
        self.filters.push(Filter {
            field: field.to_string(),
            operator: op,
            value,
        });
        self
    }

    pub fn order_by(mut self, order: &str) -> Self {
        self.order_by = Some(order.to_string());
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Build the SQL query string and bind values.
    pub fn build(&self) -> (String, Vec<Value>) {
        let fields = self
            .select_fields
            .iter()
            .map(|f| {
                if f == "*" {
                    "*".to_string()
                } else {
                    format!("\"{}\"", f)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!("SELECT {} FROM \"{}\"", fields, self.table);
        let mut bind_values = Vec::new();
        let mut param_idx = 1;

        if !self.filters.is_empty() {
            let mut clauses = Vec::new();
            for filter in &self.filters {
                match filter.operator {
                    FilterOp::IsNull => {
                        clauses.push(format!("\"{}\" IS NULL", filter.field));
                    }
                    FilterOp::IsNotNull => {
                        clauses.push(format!("\"{}\" IS NOT NULL", filter.field));
                    }
                    FilterOp::In | FilterOp::NotIn => {
                        if let Some(arr) = filter.value.as_array() {
                            let placeholders: Vec<String> = arr
                                .iter()
                                .map(|v| {
                                    let ph = format!("${}", param_idx);
                                    param_idx += 1;
                                    bind_values.push(v.clone());
                                    ph
                                })
                                .collect();
                            clauses.push(format!(
                                "\"{}\" {} ({})",
                                filter.field,
                                filter.operator.as_sql(),
                                placeholders.join(", ")
                            ));
                        }
                    }
                    _ => {
                        clauses.push(format!(
                            "\"{}\" {} ${}",
                            filter.field,
                            filter.operator.as_sql(),
                            param_idx
                        ));
                        bind_values.push(filter.value.clone());
                        param_idx += 1;
                    }
                }
            }
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }

        if let Some(ref group_by) = self.group_by {
            sql.push_str(&format!(" GROUP BY {}", group_by));
        }

        let order = self.order_by.as_deref().unwrap_or("\"modified\" DESC");
        sql.push_str(&format!(" ORDER BY {}", order));

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        } else {
            sql.push_str(" LIMIT 20");
        }

        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        (sql, bind_values)
    }
}
