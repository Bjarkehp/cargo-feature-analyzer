pub fn retry<T, E>(
    tries: usize, 
    mut request: impl FnMut() -> Result<T, E>, 
    mut error_reporter: impl FnMut(usize, E),
) -> Result<T, E> {
    for i in 1..tries {
        match request() {
            Ok(item) => return Ok(item),
            Err(e) => error_reporter(i, e)
        }
    }

    request()
}