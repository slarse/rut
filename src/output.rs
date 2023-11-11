use std::io;

/// Abstraction of an output writer used by Rut commands to write status messages.
pub trait OutputWriter {
    /// Write the content to the output.
    fn write(&mut self, content: String) -> io::Result<&mut dyn OutputWriter>;

    /// Write the content to the output and append a linefeed.
    fn writeln(&mut self, content: String) -> io::Result<&mut dyn OutputWriter> {
        self.write(content)?.linefeed()
    }

    /// Convenience method to write a linefeed to the output.
    fn linefeed(&mut self) -> io::Result<&mut dyn OutputWriter> {
        self.write(String::from("\n"))
    }

    /// Change the color of the output.
    fn set_color(&mut self, color: Color) -> io::Result<&mut dyn OutputWriter>;

    /// Change the style of the output.
    fn set_style(&mut self, stye: Style) -> io::Result<&mut dyn OutputWriter>;

    /// Reset all output formatting.
    fn reset_formatting(&mut self) -> io::Result<&mut dyn OutputWriter>;
}

/// A color used by an OutputWriter.
pub enum Color {
    Red,
    Green,
    Cyan,
    Brown,
}

/// A style used by an OutputWriter.
pub enum Style {
    Bold,
    Normal,
}
