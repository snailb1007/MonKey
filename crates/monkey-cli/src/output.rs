use serde::Serialize;
use std::io::Write;

/// CLI output format selection per D-11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

impl OutputFormat {
    /// Formats and writes the output to the specified writer.
    pub fn write_to<W: Write, T: Serialize>(
        &self,
        writer: &mut W,
        human_text: &str,
        data: &T,
    ) -> anyhow::Result<()> {
        match self {
            OutputFormat::Human => {
                writeln!(writer, "{human_text}")?;
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(data)?;
                writeln!(writer, "{json}")?;
            }
        }
        Ok(())
    }

    /// Formats and prints the output to stdout.
    pub fn print<T: Serialize>(&self, human_text: &str, data: &T) -> anyhow::Result<()> {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        self.write_to(&mut handle, human_text, data)
    }
}
