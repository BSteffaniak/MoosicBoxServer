use std::{collections::HashMap, str::FromStr as _};

use clap::{Parser, Subcommand, ValueEnum};
use itertools::Itertools;
use serde::Deserialize;
use toml::Value;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
#[clap(rename_all = "kebab_case")]
pub enum OutputType {
    Json,
    Raw,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Features {
        #[arg(index = 1)]
        file: String,

        #[arg(long)]
        offset: Option<u16>,

        #[arg(long)]
        max: Option<u16>,

        #[arg(long)]
        chunked: Option<u16>,

        #[arg(short, long)]
        spread: bool,

        #[arg(short, long, value_enum, default_value_t=OutputType::Raw)]
        output: OutputType,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    moosicbox_logging::init(None).expect("Failed to initialize logging");

    let args = Args::parse();

    match args.cmd {
        Commands::Features {
            file,
            offset,
            max,
            chunked,
            spread,
            output,
        } => {
            log::debug!("Loading file '{}'", file);
            let source = std::fs::read_to_string(file)?;
            let value: Value = toml::from_str(&source)?;

            match output {
                OutputType::Json => {
                    if let Some(workspace_members) = value
                        .get("workspace")
                        .and_then(|x| x.get("members"))
                        .and_then(|x| x.as_array())
                        .and_then(|x| x.iter().map(|x| x.as_str()).collect::<Option<Vec<_>>>())
                    {
                        let mut packages = vec![];

                        if output == OutputType::Raw {
                            panic!("workspace Cargo.toml is not supported for raw output");
                        }

                        for file in workspace_members {
                            log::debug!("Loading file '{}'", file);
                            let source = std::fs::read_to_string(format!("{}/Cargo.toml", file))?;
                            let value: Value = toml::from_str(&source)?;

                            let conf = if let Ok(path) =
                                std::path::PathBuf::from_str(&format!("{}/clippier.toml", file))
                            {
                                if path.is_file() {
                                    let source = std::fs::read_to_string(path)?;
                                    let value: ClippierConf = toml::from_str(&source)?;
                                    Some(value)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            let oses = if let Some(oses) = conf.as_ref().and_then(|x| x.os.clone())
                            {
                                oses
                            } else {
                                vec!["ubuntu-latest".to_string()]
                            };

                            log::debug!("{file} conf={conf:?}");

                            if let Some(name) = value
                                .get("package")
                                .and_then(|x| x.get("name"))
                                .and_then(|x| x.as_str())
                                .map(|x| x.to_string())
                            {
                                let features = process_features(
                                    fetch_features(&value, offset, max),
                                    chunked,
                                    spread,
                                );

                                for os in &oses {
                                    match &features {
                                        FeaturesList::Chunked(x) => {
                                            for features in x {
                                                packages.push(create_map(
                                                    os,
                                                    file,
                                                    &name,
                                                    features,
                                                    conf.as_ref(),
                                                ));
                                            }
                                        }
                                        FeaturesList::NotChunked(x) => {
                                            packages.push(create_map(
                                                os,
                                                file,
                                                &name,
                                                x,
                                                conf.as_ref(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        println!("{}", serde_json::to_value(packages).unwrap());
                    } else {
                        let features =
                            process_features(fetch_features(&value, offset, max), chunked, spread);
                        let value: serde_json::Value = features.into();
                        println!("{value}");
                    }
                }
                OutputType::Raw => {
                    let features = fetch_features(&value, offset, max);
                    if chunked.is_some() {
                        panic!("chunked arg is not supported for raw output");
                    }
                    println!("{}", features.join("\n"));
                }
            }
        }
    }

    Ok(())
}

fn create_map(
    os: &str,
    file: &str,
    name: &str,
    features: &[String],
    config: Option<&ClippierConf>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    map.insert("os".to_string(), serde_json::to_value(os).unwrap());
    map.insert("path".to_string(), serde_json::to_value(file).unwrap());
    map.insert("name".to_string(), serde_json::to_value(name).unwrap());
    map.insert("features".to_string(), features.into());

    if let Some(config) = config {
        let matches = config
            .dependencies
            .iter()
            .filter(|(_, x)| !x.os.as_ref().is_some_and(|x| x != os))
            .filter(|(_, x)| {
                !x.features.as_ref().is_some_and(|f| {
                    !f.iter()
                        .any(|required| features.iter().any(|x| x == required))
                })
            })
            .collect::<Vec<_>>();

        if !matches.is_empty() {
            map.insert(
                "dependencies".to_string(),
                serde_json::to_value(
                    matches
                        .iter()
                        .map(|(_, x)| x.command.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
                .unwrap(),
            );
        }
    }

    map
}

enum FeaturesList {
    Chunked(Vec<Vec<String>>),
    NotChunked(Vec<String>),
}

impl From<FeaturesList> for serde_json::Value {
    fn from(value: FeaturesList) -> Self {
        match value {
            FeaturesList::Chunked(x) => serde_json::to_value(x).unwrap(),
            FeaturesList::NotChunked(x) => serde_json::to_value(x).unwrap(),
        }
    }
}

fn process_features(features: Vec<String>, chunked: Option<u16>, spread: bool) -> FeaturesList {
    if let Some(chunked) = chunked {
        let count = features.len();

        FeaturesList::Chunked(if count <= chunked as usize {
            vec![features]
        } else if spread && count > 1 {
            split(&features, chunked as usize)
                .map(|x| x.to_vec())
                .collect::<Vec<_>>()
        } else {
            features
                .into_iter()
                .chunks(chunked as usize)
                .into_iter()
                .map(|x| x.collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
    } else {
        FeaturesList::NotChunked(features)
    }
}

fn fetch_features(value: &Value, offset: Option<u16>, max: Option<u16>) -> Vec<String> {
    if let Some(features) = value.get("features") {
        if let Some(features) = features.as_table() {
            let offset = offset.unwrap_or_default().into();
            let feature_count = features.keys().len() - offset;
            features
                .keys()
                .skip(offset)
                .take(
                    max.map(|x| std::cmp::min(feature_count, x as usize))
                        .unwrap_or(feature_count),
                )
                .cloned()
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

pub fn split<T>(slice: &[T], n: usize) -> impl Iterator<Item = &[T]> {
    let len = slice.len() / n;
    let rem = slice.len() % n;
    let len = if rem != 0 { len + 1 } else { len };
    let len = slice.len() / len;
    let rem = slice.len() % len;
    Split { slice, len, rem }
}

struct Split<'a, T> {
    slice: &'a [T],
    len: usize,
    rem: usize,
}

impl<'a, T> Iterator for Split<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }
        let mut len = self.len;
        if self.rem > 0 {
            len += 1;
            self.rem -= 1;
        }
        let (chunk, rest) = self.slice.split_at(len);
        self.slice = rest;
        Some(chunk)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClippierDependency {
    command: String,
    features: Option<Vec<String>>,
    os: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClippierConf {
    os: Option<Vec<String>>,
    dependencies: HashMap<String, ClippierDependency>,
}
