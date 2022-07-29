use std::env;

pub struct Config {
    pub author_name: String,
    pub author_email: String,
}

pub fn read_config() -> Result<Config, env::VarError> {
    Ok(Config {
        author_name: env::var("GIT_AUTHOR_NAME")?,
        author_email: env::var("GIT_AUTHOR_EMAIL")?,
    })
}
