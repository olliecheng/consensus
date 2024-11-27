use crate::duplicates;
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;

// include the template HTML file at compile time as a string literal
const TEMPLATE_HTML: &str = include_str!("summary_template.html");

pub fn summarize(index: &str, output: &str) -> Result<()> {
    info!("Summarising index at {index}");
    let (_, statistics, info) = duplicates::get_duplicates(index)?;

    let mut data = serde_json::to_value(info).context("Could not serialize info")?;

    println!("{}", serde_json::to_string(&statistics)?);
    data["stats"] = serde_json::Value::String(serde_json::to_string(&statistics)?);

    println!(
        "{}",
        serde_json::to_string_pretty(&data).context("Should be serialisable")?
    );

    let file = std::fs::File::create(output)?;
    let mut reg = handlebars::Handlebars::new();
    reg.render_template_to_write(TEMPLATE_HTML, &data, file);

    Ok(())
}