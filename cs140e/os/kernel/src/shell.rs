use stack_vec::StackVec;
use console::{kprint, kprintln, CONSOLE};
use std::str;

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }

    fn execute(&self) {
        match self.path() {
            "echo" => {
                let len = self.args.len();
                for s in self.args[1..len-1].iter() {
                    kprint!("{}", s);
                }
                kprintln!("{}", self.args[len-1]);
            }
            cmd => { kprintln!("unknown command: {}", cmd); }
        }
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns: it is perpetually in a shell loop.
pub fn shell(prefix: &str) -> ! {
    loop {
        let mut buf = [0u8; 128];
        let mut input = StackVec::new(&mut buf);
        kprint!("{}", prefix);

        loop {
            let byte = CONSOLE.lock().read_byte();

            if byte == b'\n' || byte == b'\r' {
                // End of input
                let mut command_storage: [&str; 64] = [""; 64];
                let result = Command::parse(
                    str::from_utf8(input.into_slice()).unwrap(),
                    &mut command_storage);

                kprint!("\n");

                match result {
                    Err(Error::TooManyArgs) => {
                        kprintln!("error: too many arguments");
                    },
                    Err(Error::Empty) => {
                        // No command, ignore.
                    }
                    Ok(command) => {
                        command.execute();
                    },
				}
                break;
            } else {
                if byte == 8 || byte == 127 {
                    if input.pop() == None {
                        CONSOLE.lock().write_byte(7 as u8);
                    } else {
                        CONSOLE.lock().write_byte(8u8);
                        CONSOLE.lock().write_byte(b' ');
                        CONSOLE.lock().write_byte(8u8);
                    }
                } else if byte < 32 || byte > 126 {
                    CONSOLE.lock().write_byte(7 as u8);
                } else {
                    if input.push(byte).is_err() {
                        kprintln!("input full!");
                        CONSOLE.lock().write_byte(7 as u8);
                    } else {
                        CONSOLE.lock().write_byte(byte);
                    }
                }
            }
        }
    }
}
