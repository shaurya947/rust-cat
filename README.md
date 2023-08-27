# rust-cat
Linux cat command written in Rust

Please note that is a simplified version of the linux `cat` command.
It supports only two flags:
1. `-n` or `--number` to number all output lines
2. `-E` or `--show-ends` to display $ at the end of each line

It correctly supports standard input using the `-` character or
when no files are specified.

It doesn't innately support wildcards. However, if the system/shell
automatically expands wildcards before passing them to the executable,
wildcards will automagically work.

The CLI interface has been separated into a binary crate while most of
the input/output processing happens in a library crate. This makes the
the IO processing more testable. There are some unit tests in lib.rs.
We use buffers to handle large files well.
