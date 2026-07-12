struct Config {
    input_file: String,
    output_file: Option<String>,
    compress: bool,
    decompress: bool,
}

fn process_args(args: Vec<String>) -> Config {
    let mut iter = args.into_iter().skip(1);

    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut compress: bool = false;
    let mut decompress: bool = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-c" => {
                compress = true;
            }
            "-x" => {
                decompress = true;
            }
            "-o" => {
                let value = iter.next().expect("-o requires an output file.");
                output = Some(value);
            }
            other => {
                input = Some(other.to_string());
            }
        }
    }

    return Config { input_file: input.expect("Input file required"), output_file: output, compress, decompress };
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name: String = args[0].clone();
    let opts: Config = process_args(args);
}
