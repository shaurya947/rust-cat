use std::io::{stdout, BufRead, BufWriter, Result, Write};

pub struct Concatenator {
    inputs: Vec<Box<dyn BufRead>>,
    add_line_numbers: bool,
    add_line_endings: bool,
}

#[derive(PartialEq)]
enum BufReadState {
    StartOfLine,
    MiddleOfLine,
}

impl Concatenator {
    pub fn new(inputs: Vec<Box<dyn BufRead>>) -> Concatenator {
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

    pub fn concatenate(self) -> Result<()> {
        use BufReadState::*;
        let mut line_count = 1;
        let mut output_stream = BufWriter::new(stdout().lock());
        for mut input in self.inputs {
            let mut buf_read_state = StartOfLine;
            'inner: loop {
                let input_buffer = input.fill_buf()?;

                // Break inner loop if this input stream is exhausted
                if input_buffer.is_empty() {
                    break 'inner;
                }

                // Add line numbers if configured, if we're at the start of a line
                if buf_read_state == StartOfLine && self.add_line_numbers {
                    write!(output_stream, "     {line_count}\t")?;
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
                    buf_read_state = MiddleOfLine;
                }

                input.consume(bytes_written);
                output_stream.flush()?;
            }
        }
        Ok(())
    }
}
