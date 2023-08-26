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
            StdIn => Ok(Box::new(io::stdin().lock())),
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

#[derive(PartialEq)]
enum BufReadState {
    StartOfLine,
    MiddleOfLine,
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
        use BufReadState::*;

        let mut line_count = 1;
        let mut output_stream = BufWriter::new(io::stdout().lock());
        'outer: for input in self.inputs {
            let input = input.get_buf_read();
            if let Err(e) = input {
                writeln!(output_stream, "cat: {e}")?;
                output_stream.flush()?;
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
                if buf_read_state == StartOfLine && self.add_line_numbers {
                    write!(
                        output_stream,
                        "{PRE_LINE_NUM_INDENT}{line_count}{POST_LINE_NUM_INDENT}"
                    )?;
                    line_count += 1;
                }

                // Write the entire buffer or until newline, whichever comes first
                let mut bytes_written =
                    output_stream.write(input_buffer.splitn(2, |b| *b == b'\n').next().unwrap())?;

                // If we didn't write the full buffer, we encountered a new line
                // Otherwise, we either hit EOF, or are in the middle of a super long line
                if bytes_written < input_buffer.len() {
                    buf_read_state = StartOfLine;

                    // Write line endings if configured
                    if self.add_line_endings {
                        write!(output_stream, "$")?;
                    }

                    // Write newline character and advance counter
                    writeln!(output_stream)?;
                    bytes_written += 1;
                } else {
                    /*
                    It's ok to set buf_read_state to MiddleOfLine even if we hit
                    EOF because once we hit EOF the inner loop breaks anyway.
                    */
                    buf_read_state = MiddleOfLine;
                }

                input.consume(bytes_written);
                output_stream.flush()?;
            }
        }
        Ok(())
    }
}
