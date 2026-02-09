use std::io::Write;

/// Writes the specified number of tabs into a instance implementing Write.
/// 
/// The function repeatably writes a single tab into the writer, to avoid dynamically allocating a string.
/// This does mean, however, that if the writer isn't buffered using something like [std::io::BufWriter],
/// this function will result in many syscalls, which is quite ineffecient. 
pub fn tab<W: Write>(writer: &mut W, depth: usize) -> std::io::Result<()> {
    for _ in 0..depth {
        write!(writer, "\t")?;
    }

    Ok(())
}