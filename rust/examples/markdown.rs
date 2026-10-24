use dumpany::context::Context;
use dumpany::core::{Dumper, DumperConfig, Text};

fn report() -> Result<String, Box<dyn std::error::Error>> {
    let mut d = Dumper::new(
        Vec::new(),
        DumperConfig {
            line_width: 48,
            ..DumperConfig::default()
        },
    );
    d.text("# Release checklist\n\n")?;

    for task in [
        "Run the Rust and Python tests before publishing the release.",
        "Build the wheels and review the package documentation.",
    ] {
        Context::new(Some("- "), None).with(&mut d, |d| d.text(task))?;
    }

    d.text("\n## Packages\n\n")?;
    for row in [
        ["Package", "Status"],
        ["---", "---"],
        ["Rust crate", "Ready"],
        ["Python extension", "Preview"],
    ] {
        Context::new(Some("| "), None).with(&mut d, |d| {
            d.join(row.map(Text), Text(" | "))?;
            d.text(" |")
        })?;
    }

    Ok(String::from_utf8(d.into_inner()?)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("{}", report()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_markdown_report() {
        assert_eq!(
            super::report().unwrap(),
            concat!(
                "# Release checklist\n\n",
                "- Run the Rust and Python tests before\n",
                "  publishing the release.\n",
                "- Build the wheels and review the package\n",
                "  documentation.\n\n",
                "## Packages\n\n",
                "| Package | Status |\n",
                "| --- | --- |\n",
                "| Rust crate | Ready |\n",
                "| Python extension | Preview |\n",
            )
        );
    }
}
