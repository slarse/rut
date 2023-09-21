use ini::Ini;
use std::env;
use std::path::{Path, PathBuf};

pub struct Config {
    pub author_name: String,
    pub author_email: String,
}

pub fn read_config() -> Result<Config, env::VarError> {
    let gitconfig = parse_gitconfig(get_gitconfig_path().unwrap()).unwrap();

    return Ok(Config {
        author_name: env::var("GIT_AUTHOR_NAME").or_else(|_| Ok(gitconfig.name.unwrap()))?,
        author_email: env::var("GIT_AUTHOR_EMAIL").or_else(|_| Ok(gitconfig.email.unwrap()))?,
    });
}

pub struct UserConfig {
    pub name: Option<String>,
    pub email: Option<String>,
}

fn get_gitconfig_path() -> Option<PathBuf> {
    let home_dir = env::var("HOME").ok()?;
    Some(PathBuf::from(home_dir).join(".gitconfig"))
}

pub fn parse_gitconfig<P: AsRef<Path>>(gitconfig_path: P) -> Result<UserConfig, ini::Error> {
    if !gitconfig_path.as_ref().is_file() {
        return Ok(UserConfig {
            name: None,
            email: None,
        });
    }

    let conf = Ini::load_from_file(&gitconfig_path)?;

    let mut user = UserConfig {
        name: None,
        email: None,
    };

    if let Some(section) = conf.section(Some("user")) {
        if let Some(name) = section.get("name") {
            user.name = Some(name.to_string());
        }
        if let Some(email) = section.get("email") {
            user.email = Some(email.to_string());
        }
    }

    Ok(user)
}
