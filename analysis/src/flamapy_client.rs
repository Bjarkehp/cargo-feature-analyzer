use std::{io::{BufRead, BufReader, BufWriter, Read, Write, stdout}, num::ParseIntError, path::Path, process::{Child, ChildStdin, ChildStdout, Command, Stdio}, string::FromUtf8Error};

use which::which;

pub struct Client {
    command: Child,
    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>,
}

impl Client {
    pub fn new(server: &Path) -> Result<Client, ConnectionError> {
        let flamapy_path = which("flamapy")?;
        let flamapy_script = std::fs::read_to_string(&flamapy_path)?;
        let python_environment_path = flamapy_script.lines()
            .next()
            .ok_or(ConnectionError::EmptyFlamapy)?
            .strip_prefix("#!")
            .ok_or(ConnectionError::NoShebang)?;

        let mut command = Command::new(python_environment_path)
            .arg(server)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let writer = command.stdin.take().unwrap();
        let reader = command.stdout.take().unwrap();
        let writer = BufWriter::new(writer);
        let reader = BufReader::new(reader);

        Ok(Client { command, writer, reader })
    }

    pub fn set_model(&mut self, path: &Path) -> Result<(), CommandError> {
        writeln!(self.writer, "set_model {:?}", path)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn estimated_number_of_configurations(&mut self) -> Result<usize, CommandError> {
        writeln!(self.writer, "estimated_number_of_configurations")?;
        self.writer.flush()?;
        let mut output = String::new();
        self.reader.read_line(&mut output)?;
        let parsed_output = output.trim().parse()
            .map_err(|e| CommandError::ParseInt(e, output))?;
        Ok(parsed_output)
    }

    pub fn configurations_number(&mut self) -> Result<usize, CommandError> {
        writeln!(self.writer, "configurations_number")?;
        self.writer.flush()?;
        let mut output = String::new();
        self.reader.read_line(&mut output)?;
        let parsed_output = output.trim().parse()
            .map_err(|e| CommandError::ParseInt(e, output))?;
        Ok(parsed_output)
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
    #[error("Flamapy error: {0}")]
    Flamapy(String),
    #[error("Unable to parse integer from flamapy output:\n\t{0}\nInput:\n\t{1}")]
    ParseInt(ParseIntError, String),
    #[error("Unable to parse bool from flamapy output:\n\t{0}")]
    ParseBool(String),
}