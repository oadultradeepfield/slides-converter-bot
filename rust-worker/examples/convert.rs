use std::{env, fs, process::ExitCode, time::Instant};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 3 {
        return Err("Usage: convert input.pdf output.pdf".into());
    }
    let input = fs::read(&arguments[1])?;
    let start = Instant::now();
    let output = slides_converter_worker::pdf::convert_pdf(input)?;
    eprintln!(
        "conversion_ms={:.3} output_bytes={}",
        start.elapsed().as_secs_f64() * 1000.0,
        output.len()
    );
    fs::write(&arguments[2], output)?;
    Ok(())
}
