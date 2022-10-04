use std::io;

/**
 * Abstraction of an output writer used by Rut commands to write status messages.
 */
pub trait OutputWriter {

    /**
     * Write the content to the output.
     */
    fn write(&mut self, content: String) -> io::Result<()>;
}
