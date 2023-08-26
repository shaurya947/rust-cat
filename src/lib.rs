use std::{
    error::Error,
    fs,
    io::{self, BufRead, BufReader, BufWriter, Write},
    writeln,
};

pub enum InputSource {
    StdIn,
    File(String),
}

impl InputSource {
    fn get_buf_read(self) -> Result<Box<dyn BufRead>, Box<dyn Error>> {
        use InputSource::*;
        match self {
            StdIn => Ok(Box::new(BufReader::new(io::stdin()))),
            File(path) => Ok(Box::new(BufReader::new(
                fs::File::open(&path).map_err(|e| format!("{path}: {e}"))?,
            ))),
        }
    }
}

pub struct Concatenator {
    inputs: Vec<InputSource>,
    add_line_numbers: bool,
    add_line_endings: bool,
}

// When printing line numbers:
// - indent 5 spaces before the number
// - indent with tab after the number
pub const PRE_LINE_NUM_INDENT: &str = "     ";
pub const POST_LINE_NUM_INDENT: &str = "\t";

impl Concatenator {
    pub fn new(inputs: Vec<InputSource>) -> Concatenator {
        Concatenator {
            inputs,
            add_line_numbers: false,
            add_line_endings: false,
        }
    }

    pub fn with_line_numbers(mut self) -> Self {
        self.add_line_numbers = true;
        self
    }

    pub fn with_line_endings(mut self) -> Self {
        self.add_line_endings = true;
        self
    }

    pub fn concatenate(self) -> io::Result<()> {
        let ins = self
            .inputs
            .into_iter()
            .map(InputSource::get_buf_read)
            .collect();

        let out = BufWriter::new(io::stdout());
        cat(ins, out, self.add_line_numbers, self.add_line_endings)
    }
}

#[derive(PartialEq)]
enum BufReadState {
    StartOfLine,
    MiddleOfLine,
}

fn cat<R, W>(
    ins: Vec<Result<R, Box<dyn Error>>>,
    mut out: W,
    line_nums: bool,
    line_ends: bool,
) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    use BufReadState::*;

    let mut line_count = 1;
    'outer: for input in ins {
        if let Err(e) = input {
            writeln!(out, "cat: {e}")?;
            out.flush()?;
            continue 'outer;
        }
        let mut input = input.unwrap();

        let mut buf_read_state = StartOfLine;
        'inner: loop {
            let input_buffer = input.fill_buf()?;

            // Break inner loop if this input stream is exhausted
            if input_buffer.is_empty() {
                break 'inner;
            }

            // Add line numbers if configured, if we're at the start of a line
            if buf_read_state == StartOfLine && line_nums {
                write!(
                    out,
                    "{PRE_LINE_NUM_INDENT}{line_count}{POST_LINE_NUM_INDENT}"
                )?;
                line_count += 1;
            }

            // Write the entire buffer or until newline, whichever comes first
            let mut bytes_written =
                out.write(input_buffer.splitn(2, |b| *b == b'\n').next().unwrap())?;

            // If we didn't write the full buffer, we encountered a new line
            // Otherwise, we either hit EOF, or are in the middle of a super long line
            if bytes_written < input_buffer.len() {
                buf_read_state = StartOfLine;

                // Write line endings if configured
                if line_ends {
                    write!(out, "$")?;
                }

                // Write newline character and advance counter
                writeln!(out)?;
                bytes_written += 1;
            } else {
                /*
                It's ok to set buf_read_state to MiddleOfLine even if we hit
                EOF because once we hit EOF the inner loop breaks anyway.
                */
                buf_read_state = MiddleOfLine;
            }

            input.consume(bytes_written);
            out.flush()?;
        }
    }
    Ok(())
}
