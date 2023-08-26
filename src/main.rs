use clap::Parser;
use exercise_3_cat::{Concatenator, InputSource};

const ABOUT: &str = r"
Concatenate FILE(s) to standard output.

With no FILE, or when FILE is -, read standard input.";

#[derive(Parser)]
#[command(version)]
#[command(about = ABOUT)]
struct Args {
    file: Vec<String>,

    /// number all output lines
    #[arg(short = 'n', long = "number")]
    show_line_numbers: bool,

    /// display $ at the end of each line
    #[arg(short = 'E', long = "show-ends")]
    show_line_ends: bool,
}

// Please note that is a simplified version of the linux `cat` command.
// It supports only two flags:
// 1. `-n` or `--number` to number all output lines
// 2. `-E` or `--show-ends` to display $ at the end of each line
//
// It correctly supports standard input using the `-` character or
// when no files are specified.
//
// It doesn't innately support wildcards. However, if the system/shell
// automatically expands wildcards before passing them to the executable,
// wildcards will automagically work.
//
// The CLI interface has been separated into a binary crate while most of
// the input/output processing happens in a library crate. This makes the
// the IO processing more testable. There are some unit tests in lib.rs.
// We use buffers to handle large files well.
fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mut inputs = args
        .file
        .into_iter()
        .map(|f| match f.as_str() {
            "-" => InputSource::StdIn,
            _ => InputSource::File(f),
        })
        .collect::<Vec<_>>();

    if inputs.is_empty() {
        inputs.push(InputSource::StdIn);
    }

    let mut catter = Concatenator::new(inputs);
    if args.show_line_numbers {
        catter = catter.with_line_numbers();
    }
    if args.show_line_ends {
        catter = catter.with_line_endings();
    }
    catter.concatenate()?;
    Ok(())
}
