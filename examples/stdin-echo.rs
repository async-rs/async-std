//! Echoes lines read on stdin to stdout.

use std::io::Write;
use async_std::io;
use async_std::task;

fn main() -> io::Result<()> {
    task::block_on(async {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let mut line = String::new();

        loop {
            // Read a line from stdin.
            let n = stdin.read_line(&mut line)?;

            // If this is the end of stdin, return.
            if n == 0 {
                return Ok(());
            }

            // Write the line to stdout.
            stdout.write_all(line.as_bytes())?;
            stdout.flush()?;
            line.clear();
        }
    })
}
