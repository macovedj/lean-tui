use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    time::Duration,
};

use serde_json::json;

use crate::{error::Result, stage::StageSize};

pub struct CastWriter {
    output: BufWriter<File>,
    clock: Duration,
    last_frame: Vec<u8>,
}

impl CastWriter {
    pub fn create(path: &Path, size: StageSize) -> Result<Self> {
        let mut output = BufWriter::new(File::create(path)?);
        let header = json!({ "version": 2, "width": size.cols, "height": size.rows });
        writeln!(output, "{header}")?;
        Ok(Self {
            output,
            clock: Duration::ZERO,
            last_frame: Vec::new(),
        })
    }

    fn write_event(&mut self, data: &str) -> Result<()> {
        let event = json!([self.clock.as_secs_f64(), "o", data]);
        Ok(writeln!(self.output, "{event}")?)
    }

    pub fn snapshot(&mut self, frame: Vec<u8>, hold: Duration) -> Result<()> {
        if frame != self.last_frame {
            self.write_event(&String::from_utf8_lossy(&frame))?;
            self.last_frame = frame;
        }
        self.clock += hold;
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.write_event("")?;
        Ok(self.output.flush()?)
    }
}
