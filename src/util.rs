
pub struct Config {
    pub input_file: String,
    pub output_file: Option<String>,
    pub compress: bool,
    pub decompress: bool,
}

pub fn print_usage(program_name: &str) {
    println!("Usage: {} [-c] [-x] [-o <output>] <input>", program_name);
}

pub fn process_args(args: Vec<String>) -> Config {
    let program_name = args[0].clone();
    let mut iter = args.into_iter().skip(1);

    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut compress: bool = false;
    let mut decompress: bool = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" => {
                print_usage(&program_name);
                std::process::exit(0);
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
                    eprintln!("Error: -o option requires an argument");
                    print_usage(&program_name);
                    std::process::exit(1);
                }
            }
            other => {
                input = Some(other.to_string());
            }
        }
    }

    let input_file = match input {
        Some(file) => file,
        None => {
            eprintln!("Error: Missing input file");
            print_usage(&program_name);
            std::process::exit(1);
        }
    };

    Config { input_file, output_file: output, compress, decompress }
}
