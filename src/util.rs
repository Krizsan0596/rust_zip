#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgError {
    Help,
    MissingOutputArg,
    ConflictingModes,
    NoModeSpecified,
    MissingInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub input_file: String,
    pub output_file: String,
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

    let output_file = match output {
        Some(file) => file,
        None => {
            return Err(ArgError::MissingOutputArg);
        }
    };

    Ok(Config {
        input_file,
        output_file,
        compress,
        decompress,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_valid_compression() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input".to_string(),
        ];
        assert_eq!(
            process_args(args),
            Ok(Config {
                input_file: "input".to_string(),
                output_file: "out".to_string(),
                compress: true,
                decompress: false,
            })
        );
    }

    #[test]
    fn test_valid_decompression() {
        let args = vec![
            "prog".to_string(),
            "-x".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input".to_string(),
        ];
        assert_eq!(
            process_args(args),
            Ok(Config {
                input_file: "input".to_string(),
                output_file: "out".to_string(),
                compress: false,
                decompress: true,
            })
        );
    }

    #[test]
    fn test_help() {
        let args = vec!["prog".to_string(), "-h".to_string()];
        assert_eq!(process_args(args), Err(ArgError::Help));
    }

    #[test]
    fn test_missing_output_arg_flag_only() {
        let args = vec!["prog".to_string(), "-c".to_string(), "-o".to_string()];
        assert_eq!(process_args(args), Err(ArgError::MissingOutputArg));
    }

    #[test]
    fn test_missing_output_arg_value() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "input".to_string(),
            "-o".to_string(),
        ];
        assert_eq!(process_args(args), Err(ArgError::MissingOutputArg));
    }

    #[test]
    fn test_conflicting_modes() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "-x".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input".to_string(),
        ];
        assert_eq!(process_args(args), Err(ArgError::ConflictingModes));
    }

    #[test]
    fn test_no_mode() {
        let args = vec![
            "prog".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input".to_string(),
        ];
        assert_eq!(process_args(args), Err(ArgError::NoModeSpecified));
    }

    #[test]
    fn test_missing_input() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "out".to_string(),
        ];
        assert_eq!(process_args(args), Err(ArgError::MissingInput));
    }

    #[test]
    fn test_repeated_positional_args() {
        let args = vec![
            "prog".to_string(),
            "-c".to_string(),
            "-o".to_string(),
            "out".to_string(),
            "input1".to_string(),
            "input2".to_string(),
        ];
        assert_eq!(
            process_args(args),
            Ok(Config {
                input_file: "input2".to_string(),
                output_file: "out".to_string(),
                compress: true,
                decompress: false,
            })
        );
    }
}
