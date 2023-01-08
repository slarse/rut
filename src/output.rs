use std::io;

/**
 * Abstraction of an output writer used by Rut commands to write status messages.
 */
pub trait OutputWriter {
    /**
     * Write the content to the output.
     */
    fn write(&mut self, content: String) -> io::Result<&mut dyn OutputWriter>;

    /**
     * Change the color of the output.
     */
    fn set_color(&mut self, color: Color) -> io::Result<&mut dyn OutputWriter>;

    /**
     * Reset all output formatting.
     */
    fn reset_formatting(&mut self) -> io::Result<&mut dyn OutputWriter>;
}

/**
 * A color used by an OutputWriter.
 */
pub enum Color {
    Red,
    Green,
}
