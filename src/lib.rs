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

        let mut out = BufWriter::new(io::stdout());
        cat(ins, &mut out, self.add_line_numbers, self.add_line_endings)
    }
}

#[derive(PartialEq)]
enum BufReadState {
    StartOfLine,
    MiddleOfLine,
}

fn cat<R, W>(
    ins: Vec<Result<R, Box<dyn Error>>>,
    out: &mut W,
    line_nums: bool,
    line_ends: bool,
) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    use BufReadState::*;

    let mut line_count = 1;
    let mut buf_read_state = StartOfLine;

    'outer: for input in ins {
        if let Err(e) = input {
            writeln!(out, "cat: {e}")?;
            out.flush()?;
            buf_read_state = StartOfLine;
            continue 'outer;
        }

        let mut input = input.unwrap();
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
                buf_read_state = MiddleOfLine;
            }

            input.consume(bytes_written);
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod cat_tests {
    use std::{
        io::{self, Cursor},
        str,
    };

    use crate::{POST_LINE_NUM_INDENT, PRE_LINE_NUM_INDENT};

    use super::cat;

    const INPUT_STREAM_1: &str = "This is the first file...
Second line of first file now
Not ending with a new line";
    const INPUT_STREAM_2: &str = "This is the second file...
Second line of second file now
Going to end with a new line
";
    const INPUT_STREAM_3: &str = "This is the third file...
Second line of third file now
Not ending with a new line";

    const ERROR_1: &str = "Oops, something went wrong!";

    #[test]
    fn no_ins_no_out() -> io::Result<()> {
        let ins = vec![Ok(Cursor::new(String::new()))];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, false, false)?;

        assert_eq!(out.len(), 0);
        Ok(())
    }

    #[test]
    fn one_in_correct_out() -> io::Result<()> {
        let ins = vec![Ok(Cursor::new(INPUT_STREAM_1))];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, false, false)?;

        assert_eq!(str::from_utf8(&out).unwrap(), INPUT_STREAM_1);
        Ok(())
    }

    #[test]
    fn one_in_error_correct_out() -> io::Result<()> {
        let ins: Vec<Result<Cursor<Vec<u8>>, _>> = vec![Err(ERROR_1.into())];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, false, false)?;

        assert_eq!(str::from_utf8(&out).unwrap(), format!("cat: {ERROR_1}\n"));
        Ok(())
    }

    #[test]
    fn multiple_ins_correct_out() -> io::Result<()> {
        let ins = vec![
            Ok(Cursor::new(INPUT_STREAM_1)),
            Ok(Cursor::new(INPUT_STREAM_2)),
            Ok(Cursor::new(INPUT_STREAM_3)),
        ];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, false, false)?;

        assert_eq!(
            str::from_utf8(&out).unwrap(),
            format!("{INPUT_STREAM_1}{INPUT_STREAM_2}{INPUT_STREAM_3}")
        );
        Ok(())
    }

    #[test]
    fn multiple_ins_with_error_correct_out() -> io::Result<()> {
        let ins = vec![
            Ok(Cursor::new(INPUT_STREAM_1)),
            Err(ERROR_1.into()),
            Ok(Cursor::new(INPUT_STREAM_2)),
            Ok(Cursor::new(INPUT_STREAM_3)),
        ];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, false, false)?;

        assert_eq!(
            str::from_utf8(&out).unwrap(),
            format!("{INPUT_STREAM_1}cat: {ERROR_1}\n{INPUT_STREAM_2}{INPUT_STREAM_3}")
        );
        Ok(())
    }

    #[test]
    fn line_nums_correct_out() -> io::Result<()> {
        let ins = vec![
            Ok(Cursor::new(INPUT_STREAM_1)),
            Ok(Cursor::new(INPUT_STREAM_2)),
            Ok(Cursor::new(INPUT_STREAM_3)),
        ];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, true, false)?;

        let (lines_1, lines_2, lines_3) = (
            INPUT_STREAM_1.lines().collect::<Vec<_>>(),
            INPUT_STREAM_2.lines().collect::<Vec<_>>(),
            INPUT_STREAM_3.lines().collect::<Vec<_>>(),
        );

        let expected_out = vec![
            format!(
                "{PRE_LINE_NUM_INDENT}1{POST_LINE_NUM_INDENT}{}\n",
                lines_1[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}2{POST_LINE_NUM_INDENT}{}\n",
                lines_1[1]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}3{POST_LINE_NUM_INDENT}{}{}\n",
                lines_1[2], lines_2[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}4{POST_LINE_NUM_INDENT}{}\n",
                lines_2[1]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}5{POST_LINE_NUM_INDENT}{}\n",
                lines_2[2]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}6{POST_LINE_NUM_INDENT}{}\n",
                lines_3[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}7{POST_LINE_NUM_INDENT}{}\n",
                lines_3[1]
            ),
            format!("{PRE_LINE_NUM_INDENT}8{POST_LINE_NUM_INDENT}{}", lines_3[2]),
        ];

        assert_eq!(str::from_utf8(&out).unwrap(), expected_out.join(""));
        Ok(())
    }

    #[test]
    fn line_nums_with_error_correct_out() -> io::Result<()> {
        let ins = vec![
            Ok(Cursor::new(INPUT_STREAM_1)),
            Err(ERROR_1.into()),
            Ok(Cursor::new(INPUT_STREAM_2)),
            Ok(Cursor::new(INPUT_STREAM_3)),
        ];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, true, false)?;

        let (lines_1, lines_2, lines_3) = (
            INPUT_STREAM_1.lines().collect::<Vec<_>>(),
            INPUT_STREAM_2.lines().collect::<Vec<_>>(),
            INPUT_STREAM_3.lines().collect::<Vec<_>>(),
        );

        let expected_out = vec![
            format!(
                "{PRE_LINE_NUM_INDENT}1{POST_LINE_NUM_INDENT}{}\n",
                lines_1[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}2{POST_LINE_NUM_INDENT}{}\n",
                lines_1[1]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}3{POST_LINE_NUM_INDENT}{}cat: {ERROR_1}\n",
                lines_1[2]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}4{POST_LINE_NUM_INDENT}{}\n",
                lines_2[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}5{POST_LINE_NUM_INDENT}{}\n",
                lines_2[1]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}6{POST_LINE_NUM_INDENT}{}\n",
                lines_2[2]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}7{POST_LINE_NUM_INDENT}{}\n",
                lines_3[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}8{POST_LINE_NUM_INDENT}{}\n",
                lines_3[1]
            ),
            format!("{PRE_LINE_NUM_INDENT}9{POST_LINE_NUM_INDENT}{}", lines_3[2]),
        ];

        assert_eq!(str::from_utf8(&out).unwrap(), expected_out.join(""));
        Ok(())
    }

    #[test]
    fn line_ends_correct_out() -> io::Result<()> {
        let ins = vec![
            Ok(Cursor::new(INPUT_STREAM_1)),
            Ok(Cursor::new(INPUT_STREAM_2)),
            Ok(Cursor::new(INPUT_STREAM_3)),
        ];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, false, true)?;

        let (lines_1, lines_2, lines_3) = (
            INPUT_STREAM_1.lines().collect::<Vec<_>>(),
            INPUT_STREAM_2.lines().collect::<Vec<_>>(),
            INPUT_STREAM_3.lines().collect::<Vec<_>>(),
        );

        let expected_out = vec![
            format!("{}$\n", lines_1[0]),
            format!("{}$\n", lines_1[1]),
            format!("{}{}$\n", lines_1[2], lines_2[0]),
            format!("{}$\n", lines_2[1]),
            format!("{}$\n", lines_2[2]),
            format!("{}$\n", lines_3[0]),
            format!("{}$\n", lines_3[1]),
            format!("{}", lines_3[2]),
        ];

        assert_eq!(str::from_utf8(&out).unwrap(), expected_out.join(""));
        Ok(())
    }

    #[test]
    fn line_ends_with_error_correct_out() -> io::Result<()> {
        let ins = vec![
            Ok(Cursor::new(INPUT_STREAM_1)),
            Err(ERROR_1.into()),
            Ok(Cursor::new(INPUT_STREAM_2)),
            Ok(Cursor::new(INPUT_STREAM_3)),
        ];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, false, true)?;

        let (lines_1, lines_2, lines_3) = (
            INPUT_STREAM_1.lines().collect::<Vec<_>>(),
            INPUT_STREAM_2.lines().collect::<Vec<_>>(),
            INPUT_STREAM_3.lines().collect::<Vec<_>>(),
        );

        let expected_out = vec![
            format!("{}$\n", lines_1[0]),
            format!("{}$\n", lines_1[1]),
            format!("{}cat: {ERROR_1}\n", lines_1[2]),
            format!("{}$\n", lines_2[0]),
            format!("{}$\n", lines_2[1]),
            format!("{}$\n", lines_2[2]),
            format!("{}$\n", lines_3[0]),
            format!("{}$\n", lines_3[1]),
            format!("{}", lines_3[2]),
        ];

        assert_eq!(str::from_utf8(&out).unwrap(), expected_out.join(""));
        Ok(())
    }

    #[test]
    fn line_nums_and_ends_correct_out() -> io::Result<()> {
        let ins = vec![
            Ok(Cursor::new(INPUT_STREAM_1)),
            Ok(Cursor::new(INPUT_STREAM_2)),
            Ok(Cursor::new(INPUT_STREAM_3)),
        ];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, true, true)?;

        let (lines_1, lines_2, lines_3) = (
            INPUT_STREAM_1.lines().collect::<Vec<_>>(),
            INPUT_STREAM_2.lines().collect::<Vec<_>>(),
            INPUT_STREAM_3.lines().collect::<Vec<_>>(),
        );

        let expected_out = vec![
            format!(
                "{PRE_LINE_NUM_INDENT}1{POST_LINE_NUM_INDENT}{}$\n",
                lines_1[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}2{POST_LINE_NUM_INDENT}{}$\n",
                lines_1[1]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}3{POST_LINE_NUM_INDENT}{}{}$\n",
                lines_1[2], lines_2[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}4{POST_LINE_NUM_INDENT}{}$\n",
                lines_2[1]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}5{POST_LINE_NUM_INDENT}{}$\n",
                lines_2[2]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}6{POST_LINE_NUM_INDENT}{}$\n",
                lines_3[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}7{POST_LINE_NUM_INDENT}{}$\n",
                lines_3[1]
            ),
            format!("{PRE_LINE_NUM_INDENT}8{POST_LINE_NUM_INDENT}{}", lines_3[2]),
        ];

        assert_eq!(str::from_utf8(&out).unwrap(), expected_out.join(""));
        Ok(())
    }

    #[test]
    fn line_nums_and_ends_with_error_correct_out() -> io::Result<()> {
        let ins = vec![
            Ok(Cursor::new(INPUT_STREAM_1)),
            Err(ERROR_1.into()),
            Ok(Cursor::new(INPUT_STREAM_2)),
            Ok(Cursor::new(INPUT_STREAM_3)),
        ];
        let mut out = Vec::<u8>::default();
        cat(ins, &mut out, true, true)?;

        let (lines_1, lines_2, lines_3) = (
            INPUT_STREAM_1.lines().collect::<Vec<_>>(),
            INPUT_STREAM_2.lines().collect::<Vec<_>>(),
            INPUT_STREAM_3.lines().collect::<Vec<_>>(),
        );

        let expected_out = vec![
            format!(
                "{PRE_LINE_NUM_INDENT}1{POST_LINE_NUM_INDENT}{}$\n",
                lines_1[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}2{POST_LINE_NUM_INDENT}{}$\n",
                lines_1[1]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}3{POST_LINE_NUM_INDENT}{}cat: {ERROR_1}\n",
                lines_1[2]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}4{POST_LINE_NUM_INDENT}{}$\n",
                lines_2[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}5{POST_LINE_NUM_INDENT}{}$\n",
                lines_2[1]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}6{POST_LINE_NUM_INDENT}{}$\n",
                lines_2[2]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}7{POST_LINE_NUM_INDENT}{}$\n",
                lines_3[0]
            ),
            format!(
                "{PRE_LINE_NUM_INDENT}8{POST_LINE_NUM_INDENT}{}$\n",
                lines_3[1]
            ),
            format!("{PRE_LINE_NUM_INDENT}9{POST_LINE_NUM_INDENT}{}", lines_3[2]),
        ];

        assert_eq!(str::from_utf8(&out).unwrap(), expected_out.join(""));
        Ok(())
    }
}
