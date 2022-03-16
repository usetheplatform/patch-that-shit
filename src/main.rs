use std::env;
use std::fs::File;
use std::process;
use serde::{Deserialize, Serialize};
use serde_yaml;
use serde_json;
use json_patch;


// Remove json_patch and serde_diff and use treediff, remove json <-> yaml logic
// Format the resulting yaml
// TODO: Github integration
// when it's done it's possible to create a PR to the repository, read GITHUB_TOKEN from env
fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    match run(&config) {
        Ok(_) => println!("All done"),
        Err(ApplicationError(err)) => panic!("{}", err),
    };
}

fn run(config: &Config) -> Result<(), ApplicationError> {
    match &config.command {
        Command::GeneratePatch(path_to_origin, path_to_reference) => {
            generate_patch(&path_to_origin, &path_to_reference)
        }

        Command::ApplyPatch(path_to_patch, path_to_origin) => {
            apply_patch(&path_to_patch, &path_to_origin)
        }
        Command::DistributePatch(repositories, github_token, path_to_patch, path_to_origin) => {
            panic!("Command::DistributePatch is not implemented yet");
        }
    }
}

enum Command {
    // Path to Origin, Path to Patched
    GeneratePatch(String, String),
    // Path to Patch, Path  to File
    ApplyPatch(String, String),
    // Repositores, Token, Path to Patch
    DistributePatch(Vec<String>, String, String, String)
}

struct Config {
    command: Command
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }

        let command = args[1].clone();

        
        let config = match command.as_str() {
            "generate-patch" => {
                if args.len() < 4 {
                    return Err("not enough arguments");
                }
                // do not rely on order, but rather on a name
                let path_to_origin = args[2].clone();
                let path_to_reference = args[3].clone();
                Ok(Config { command: Command::GeneratePatch(path_to_origin, path_to_reference) })
            },
            "apply-patch" => {
                if args.len() < 4 {
                    return Err("not enough arguments");
                }

                // do not rely on order, but rather on a name
                let path_to_patch = args[2].clone();
                let path_to_origin = args[3].clone();

                Ok(Config { command: Command::ApplyPatch(path_to_patch, path_to_origin) })
            },
            "distribute-patch" => {
                if args.len() < 5 {
                    return Err("not enough arguments");
                }

                // do not rely on order, but rather on a name
                let repositories: Vec<String> = args[2].clone().split_whitespace().map(str::to_string).collect();
                let path_to_patch = args[3].clone();
                let path_to_origin = args[4].clone();
                let github_token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN is mandatory for this command");


                Ok(Config { command: Command::DistributePatch(repositories, github_token, path_to_patch, path_to_origin) })
            },
            _ => Err("unexpected command passed")
        };

        config
    }
}

#[derive(Debug)]
struct ApplicationError(String);

fn read_yaml(filename: &String) -> Result<serde_yaml::Value, serde_yaml::Error> {
    let file = File::open(filename).expect(&format!("File {} not found", stringify!(filename)));

    serde_yaml::from_reader(&file)
}

fn write_yaml(filename: &String, yaml: &serde_yaml::Value) -> Result<(), serde_yaml::Error> {
    let file = File::create(filename).expect(&format!("File {} not found", stringify!(filename)));

   serde_yaml::to_writer(file, &yaml)
}

fn write_json(filename: &String, json: &serde_json::Value) -> Result<(), serde_json::Error> {
    let file = File::create(filename).expect(&format!("File {} not found", stringify!(filename)));

    serde_json::to_writer_pretty(file, json)

}

fn read_json(filename: &String) -> Result<serde_json::Value, serde_json::Error> {
    let file = File::open(filename).expect(&format!("File {} not found", stringify!(filename)));

    serde_json::from_reader(&file)
}

fn json_to_yaml(json: serde_json::Value) -> Result<serde_yaml::Value, serde_json::Error> {
    serde_json::from_value::<serde_yaml::Value>(json)
}

fn yaml_to_json(yaml: serde_yaml::Value) -> Result<serde_json::Value, serde_yaml::Error> {
    serde_yaml::from_value::<serde_json::Value>(yaml)
}

fn generate_patch(path_to_origin: &String, path_to_reference: &String) -> Result<(), ApplicationError> {
    // let origin = read_yaml(path_to_origin).and_then(yaml_to_json).map_err(|err| { ApplicationError(format!("Failed to read file: {}, {}", path_to_origin, err)) } )?;
    // let reference = read_yaml(path_to_reference).and_then(yaml_to_json).map_err(|err| { ApplicationError(format!("Failed to read file: {}, {}", path_to_reference, err)) } )?;

    // let patch = json_patch::diff(&origin, &reference);

    // serde_json::to_writer_pretty(File::create(&String::from("fixtures/basic-patch.json")).expect("фы"), &patch);

    // println!("Generated patch: {:?}", patch); // prettify

    let origin = read_yaml(path_to_origin).map_err(|err| { ApplicationError(format!("Failed to read file: {}, {}", path_to_origin, err)) } )?;
    let reference = read_yaml(path_to_reference).map_err(|err| { ApplicationError(format!("Failed to read file: {}, {}", path_to_reference, err)) } )?;

    merge(&origin, &reference);

    Ok(())
}

fn apply_patch(path_to_patch: &String, path_to_origin: &String) -> Result<(), ApplicationError> {
    // Do a better error handling
    let mut origin = read_yaml(path_to_origin).and_then(yaml_to_json).map_err(|err| { ApplicationError(format!("Failed to read file: {}, {}", path_to_patch, err)) } )?;
    let patch = read_json(path_to_patch).and_then(json_patch::from_value).map_err(|err| { ApplicationError(format!("Failed to read file: {}, {}", path_to_patch, err)) } )?;

    json_patch::patch(&mut origin, &patch).map_err(|err| { ApplicationError(format!("Failed to apply patch from {} to {}", path_to_patch, path_to_origin)) });


    let origin_yaml = json_to_yaml(origin).expect("Error");

    write_yaml(&String::from("fixtures/result.yaml"), &origin_yaml).map_err(|err| { ApplicationError(format!("Failed to write file: {}, {}", path_to_origin, err)) } )
}

fn diff(a: &serde_yaml::Value, b: &serde_yaml::Value) {
    let mut recorder = treediff::tools::Recorder::default();

    treediff::diff(a, b, &mut recorder);

    println!("{:?}", recorder.calls);
}

fn merge(a: &serde_yaml::Value, b: &serde_yaml::Value) {
    let mut merger = treediff::tools::Merger::from(serde_yaml::Value::clone(a));

    treediff::diff(a, b, &mut merger);

    println!("{:?}", merger.into_inner());

    write_yaml(&String::from("fixtures/result.yaml"), b).map_err(|err| { ApplicationError(format!("Failed to write file")) } );
}