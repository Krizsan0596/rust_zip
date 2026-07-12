
pub enum ArgError {
    Help,
    MissingOutputArg,
    ConflictingModes,
    NoModeSpecified,
    MissingInput,
}

pub struct Config {
    pub input_file: String,
    pub output_file: Option<String>,
    pub compress: bool,
    pub decompress: bool,
}

pub fn print_usage(program_name: &str) {
    println!("Usage: {} [-c] [-x] [-o <output>] <input>", program_name);
}

pub fn process_args(args: Vec<String>) -> Result<Config, ArgError> {
    let mut iter = args.into_iter().skip(1);

    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut compress: bool = false;
    let mut decompress: bool = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" => {
                return Err(ArgError::Help);
            }
            "-c" => {
                compress = true;
            }
            "-x" => {
                decompress = true;
            }
            "-o" => {
                if let Some(value) = iter.next() {
                    output = Some(value);
                } else {
                    return Err(ArgError::MissingOutputArg);
                }
            }
            other => {
                input = Some(other.to_string());
            }
        }
    }

    if compress && decompress {
        return Err(ArgError::ConflictingModes);
    }

    if !compress && !decompress {
        return Err(ArgError::NoModeSpecified);
    }

    let input_file = match input {
        Some(file) => file,
        None => {
            return Err(ArgError::MissingInput);
        }
    };

    Ok(Config { input_file, output_file: output, compress, decompress })
}
