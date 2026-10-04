use std::{
    fs::File,
    io::{self, Write},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct LogWriter(pub Arc<Mutex<File>>);
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
impl Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let _ = io::stdout().write_all(bytes);
        self.0.lock().unwrap().write_all(bytes)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()?;
        self.0.lock().unwrap().flush()
    }
}
