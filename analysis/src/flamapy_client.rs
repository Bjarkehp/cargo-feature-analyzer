use std::{io::{BufRead, BufReader, BufWriter, Write}, num::ParseFloatError, path::Path, process::{ChildStdin, ChildStdout, Command, Stdio}};

use which::which;

pub struct Client {
    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>,
}

impl Client {
    pub fn new(server: impl AsRef<Path>) -> Result<Client, ConnectionError> {
        let flamapy_path = which("flamapy")?;
        let flamapy_script = std::fs::read_to_string(&flamapy_path)?;
        let python_environment_path = flamapy_script.lines()
            .next()
            .ok_or(ConnectionError::EmptyFlamapy)?
            .strip_prefix("#!")
            .ok_or(ConnectionError::NoShebang)?;

        let mut command = Command::new(python_environment_path)
            .arg(server.as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let writer = command.stdin.take().unwrap();
        let reader = command.stdout.take().unwrap();
        let writer = BufWriter::new(writer);
        let reader = BufReader::new(reader);

        Ok(Client { writer, reader })
    }

    pub fn set_model(&mut self, path: &Path) -> Result<(), CommandError> {
        writeln!(self.writer, "set_model {:?}", path)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn estimated_number_of_configurations(&mut self) -> Result<f64, CommandError> {
        writeln!(self.writer, "estimated_number_of_configurations")?;
        self.writer.flush()?;
        let mut output = String::new();
        self.reader.read_line(&mut output)?;
        output.trim().parse()
            .map_err(|e| CommandError::ParseFloat(e, output))
    }

    pub fn configurations_number(&mut self) -> Result<f64, CommandError> {
        writeln!(self.writer, "configurations_number")?;
        self.writer.flush()?;
        let mut output = String::new();
        self.reader.read_line(&mut output)?;
        output.trim().parse()
            .map_err(|e| CommandError::ParseFloat(e, output))
    }

    pub fn satisfiable_configuration(&mut self, configuration_path: &Path) -> Result<bool, CommandError> {
        writeln!(self.writer, "satisfiable_configuration {:?}", configuration_path)?;
        self.writer.flush()?;
        let mut output = String::new();
        self.reader.read_line(&mut output)?;
        match output.trim() {
            "True" => Ok(true),
            "False" => Ok(false),
            _ => Err(CommandError::ParseBool(output))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Which(#[from] which::Error),
    #[error("Flamapy script was empty")]
    EmptyFlamapy,
    #[error("Flamapy script did not contain a shebang")]
    NoShebang,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("IO error while calling flamapy")]
    Io(#[from] std::io::Error),
    #[error("Unable to parse float from flamapy output:\n\t{0}\nInput:\n\t{1}")]
    ParseFloat(ParseFloatError, String),
    #[error("Unable to parse bool from flamapy output:\n\t{0}")]
    ParseBool(String),
}