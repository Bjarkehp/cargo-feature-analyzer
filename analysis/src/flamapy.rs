use std::{num::ParseIntError, path::Path, process::Command, str::ParseBoolError, string::FromUtf8Error};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error while calling flamapy")]
    Io(#[from] std::io::Error),
    #[error("Flamapy error: {0}")]
    Flamapy(String),
    #[error("Flamapy output was not UTF-8")]
    Utf8(#[from] FromUtf8Error),
    #[error("Unable to parse integer from flamapy output:\n\t{0}\nInput:\n\t{1}")]
    ParseInt(ParseIntError, String),
    #[error("Unable to parse bool from flamapy output:\n\t{0}")]
    ParseBool(String),
}

pub type Result<T> = std::result::Result<T, Error>;

fn run_flamapy_command(command_map: impl FnOnce(&mut Command) -> &mut Command) -> Result<String> {
    let mut command = Command::new("flamapy");
    let command_with_args = command_map(&mut command);
    let output = command_with_args.output()?;
    
    if !output.stderr.is_empty() {
        println!("{:?}", command);
        Err(Error::Flamapy(String::from_utf8(output.stderr)?))
    } else {
        let output_string = String::from_utf8(output.stdout)?;
        Ok(output_string)
    }
}

pub fn estimated_number_of_configurations(path: &Path) -> Result<usize> {
    let output = run_flamapy_command(|c| c.arg("estimated_number_of_configurations").arg(path))?;
    let number = output.trim().parse::<usize>()
        .map_err(|e| Error::ParseInt(e, output))?;
    Ok(number)
}

pub fn configurations_number(path: &Path) -> Result<usize> {
    let output = run_flamapy_command(|c| c.arg("configurations_number").arg(path))?;
    let number = output.trim().parse::<usize>()
        .map_err(|e| Error::ParseInt(e, output))?;
    Ok(number)
}

pub fn satisfiable_configuration(model_path: &Path, configuration_path: &Path) -> Result<bool> {
    let output = run_flamapy_command(|c| c.arg("satisfiable_configuration").arg(model_path).arg(configuration_path))?;
    match output.trim() {
        "True" => Ok(true),
        "False" => Ok(false),
        _ => return Err(Error::ParseBool(output)),
    }
} 