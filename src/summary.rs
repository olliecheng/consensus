use crate::duplicates;
use anyhow::{Context, Result};
use serde_json::json;

// include the template HTML file at compile time as a string literal
const TEMPLATE_HTML: &str = include_str!("summary_template.html");

pub fn summarize(index: &str, output: &str) -> Result<()> {
    info!("Summarising index at {index}");
    let (_, statistics, info) = duplicates::get_duplicates(index)?;
    let gb = info.gb;

    let mut data = serde_json::to_value(info).context("Could not serialize info")?;

    println!("{}", serde_json::to_string(&statistics)?);
    // round "gb" stat to 3dp
    data["gb"] = json!(format!("{:.3}", gb));
    data["stats"] = json!(serde_json::to_string(&statistics)?);

    println!(
        "{}",
        serde_json::to_string_pretty(&data).context("Should be serialisable")?
    );

    let file = std::fs::File::create(output)?;
    let reg = handlebars::Handlebars::new();
    reg.render_template_to_write(TEMPLATE_HTML, &data, file)?;

    Ok(())
}