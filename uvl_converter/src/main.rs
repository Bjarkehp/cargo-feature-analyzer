use std::{fs::File, io::{BufReader, BufWriter, Read, Write}, path::PathBuf};

use anyhow::{Context};
use clap::Parser;
use itertools::Itertools;
use xml::{EventReader, reader::XmlEvent};

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    #[arg(long)]
    xml_model: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    features: Option<PathBuf>,
    #[arg(long)]
    destination: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let xml_model = File::open(&args.xml_model)?;
    let mut reader = BufReader::new(xml_model);

    let uvl_model = File::create(&args.destination)?;
    let mut writer = BufWriter::new(uvl_model);

    xml_to_uvl(&mut reader, &mut writer, "linux")
        .with_context(|| format!("Error while converting {}", args.xml_model.display()))?;

    writer.flush()?;

    Ok(())
}

fn xml_to_uvl<R: Read, W: Write>(reader: &mut R, writer: &mut W, root: &str) -> anyhow::Result<()> {
    let mut parser = EventReader::new(reader);
    let (features, cnf) = parse_features_and_cnf(&mut parser)
        .with_context(|| "Error while parsing")?;
    write_features_and_cnf_to_uvl(writer, &features, &cnf, root)
        .with_context(|| "Error while writing")?;
    Ok(())
}

type Disjunction = Vec<(String, bool)>;

fn parse_features_and_cnf<R: Read>(parser: &mut EventReader<R>) -> anyhow::Result<(Vec<String>, Vec<Disjunction>)> {
    let mut features = vec![];
    let mut disjunctions = vec![];
    
    loop {
        match parser.next()? {
            XmlEvent::StartElement { name, mut attributes, .. } => {
                match name.local_name.as_str() {
                    "feature" => {
                        let name = attributes.pop()
                            .filter(|a| a.name.local_name == "name")
                            .with_context(|| "Feature attribute's first attribute was not a name")?
                            .value;
                        features.push(name);
                    },
                    "rule" => {
                        let rule = parse_disjunction(parser)?;
                        disjunctions.push(rule);
                    }
                    _ => {}
                }
            },
            XmlEvent::EndDocument => return Ok((features, disjunctions)),
            _ => {}
        }
    }
}

fn parse_disjunction<R: Read>(parser: &mut EventReader<R>) -> xml::reader::Result<Vec<(String, bool)>> {
    let mut disjunction = vec![];
    let mut negated = false;

    loop {
        match parser.next()? {
            XmlEvent::StartElement { name, .. } => {
                if name.local_name == "not" {
                    negated = true;
                } 
            }
            XmlEvent::Characters(content) => {
                disjunction.push((content, !negated))
            },
            XmlEvent::EndElement { name } => {
                if name.local_name == "rule" {
                    return Ok(disjunction);
                }
            }
            _ => {}
        }
    }
}

fn write_features_and_cnf_to_uvl<W: Write>(writer: &mut W, features: &[String], cnf: &[Disjunction], root: &str) -> std::io::Result<()> {
    writeln!(writer, "features")?;
    writeln!(writer, "\t{root}")?;
    writeln!(writer, "\t\toptional")?;
    for feature in features {
        writeln!(writer, "\t\t\t\"{feature}\"")?;
    }

    writeln!(writer, "constraints")?;

    for disjunction in cnf {
        write!(writer, "\t")?;
        let formatted_disjunction = disjunction.iter()
            .map(|(var, assignment)| format_litteral(var, *assignment))
            .join(" | ");
        writeln!(writer, "{}", formatted_disjunction)?;
    }

    Ok(())
}

fn format_litteral(var: &str, assignment: bool) -> String {
    if assignment {
        format!("\"{var}\"")
    } else {
        format!("!\"{var}\"")
    }
}