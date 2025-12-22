use std::{fs::File, io::{BufWriter, Write}, path::Path};

use anyhow::Context;

pub fn write<T>(
    path: &Path, 
    data: impl Iterator<Item = T>, 
    columns: &[&str], 
    write_fn: impl Fn(&mut BufWriter<File>, T) -> std::io::Result<()>
) -> anyhow::Result<()> {
    {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        
        write!(writer, "{}", columns[0])?;
        for column in &columns[1..] {
            write!(writer, ",{}", column)?;
        }
        writeln!(writer)?;

        for item in data {
            write_fn(&mut writer, item)?;
        }

        writer.flush()?;

        Ok::<(), std::io::Error>(())
    }.with_context(|| format!("Failed to write results into {path:?}"))
}