use rhai::{Dynamic, Engine};

/// Register the loom API functions into the Rhai engine.
/// Functions are registered as global functions callable from anywhere in scripts,
/// including inside `fn` definitions.
///
/// Available in scripts as: throw(msg), today(), now(), log(msg), msgprint(msg), date_diff(d1, d2)
pub fn register_loom_api(engine: &mut Engine) {
    // throw(msg) — raise a validation error
    engine.register_fn(
        "throw",
        |msg: &str| -> Result<(), Box<rhai::EvalAltResult>> { Err(msg.into()) },
    );

    // msgprint(msg) — flash message (logged server-side)
    engine.register_fn("msgprint", |msg: &str| {
        tracing::info!("[msgprint] {}", msg);
    });

    // log(msg) — server log
    engine.register_fn("log", |msg: &str| {
        tracing::info!("[script] {}", msg);
    });

    // today() — current date as "YYYY-MM-DD"
    engine.register_fn("today", || -> String {
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    });

    // now() — current datetime as ISO string
    engine.register_fn("now", || -> String { chrono::Utc::now().to_rfc3339() });

    // date_diff(date1, date2) — days between two date strings
    engine.register_fn("date_diff", |date1: &str, date2: &str| -> Dynamic {
        let d1 = chrono::NaiveDate::parse_from_str(date1, "%Y-%m-%d");
        let d2 = chrono::NaiveDate::parse_from_str(date2, "%Y-%m-%d");
        match (d1, d2) {
            (Ok(d1), Ok(d2)) => Dynamic::from((d1 - d2).num_days()),
            _ => Dynamic::from(0_i64),
        }
    });
}
