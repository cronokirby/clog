use anyhow::anyhow;
use std::path::PathBuf;

/// A static string for usage errors.
const USAGE: &str = "usage: clog <input_dir> <output_dir>";

/// Arguments to the program.
#[derive(Debug)]
struct Program {
    /// The input directory for the blog's files.
    input_dir: PathBuf,
    /// Where the site should be generated.
    output_dir: PathBuf,
}

impl Program {
    fn parse() -> anyhow::Result<Self> {
        let mut args = std::env::args().skip(1);
        Ok(Self {
            input_dir: args.next().ok_or_else(|| anyhow!(USAGE))?.into(),
            output_dir: args.next().ok_or_else(|| anyhow!(USAGE))?.into(),
        })
    }

    fn run(self) -> anyhow::Result<()> {
        println!("input_dir: {:?}", self.input_dir);
        println!("output_dir: {:?}", self.output_dir);
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let prog = Program::parse()?;
    prog.run()?;
    Ok(())
}
