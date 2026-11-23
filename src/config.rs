use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub type Bookmarks = BTreeMap<String, PathBuf>;

#[derive(Debug)]
pub enum ConfigError {
    NoHome,
    Io(io::Error),
    InvalidName(String),
    NotFound(String),
    AlreadyExists(String, PathBuf),
    InvalidLine(usize, String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHome => write!(f, "home directory not found"),
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::InvalidName(name) => {
                write!(
                    f,
                    "invalid bookmark name '{name}': use letters, digits, '-', or '_'"
                )
            }
            Self::NotFound(name) => write!(f, "bookmark '{name}' not found"),
            Self::AlreadyExists(name, path) => {
                write!(
                    f,
                    "bookmark '{name}' already exists (path: {})",
                    path.display()
                )
            }
            Self::InvalidLine(n, line) => write!(f, "invalid config line {n}: {line}"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

fn home_dir() -> Result<PathBuf, ConfigError> {
    env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .ok_or(ConfigError::NoHome)
}

pub fn config_dir() -> Result<PathBuf, ConfigError> {
    Ok(home_dir()?.join(".tp"))
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    Ok(config_dir()?.join("tp.conf"))
}

pub fn validate_name(name: &str) -> Result<(), ConfigError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ConfigError::InvalidName(name.to_string()));
    }
    Ok(())
}

pub fn expand_path(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir().unwrap_or_else(|_| PathBuf::from("~"));
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

pub fn load() -> Result<Bookmarks, ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Bookmarks::new());
    }

    let contents = fs::read_to_string(&path)?;
    let mut bookmarks = Bookmarks::new();

    for (idx, raw) in contents.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            return Err(ConfigError::InvalidLine(idx + 1, raw.to_string()));
        };
        let name = name.trim();
        let value = value.trim();
        validate_name(name)?;
        bookmarks.insert(name.to_string(), PathBuf::from(value));
    }

    Ok(bookmarks)
}

pub fn save(bookmarks: &Bookmarks) -> Result<(), ConfigError> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir)?;

    let path = config_path()?;
    let tmp = dir.join("tp.conf.tmp");

    {
        let mut file = fs::File::create(&tmp)?;
        writeln!(file, "# tp — teleport bookmarks")?;
        for (name, path) in bookmarks {
            writeln!(file, "{name}={}", path.display())?;
        }
    }

    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Add a bookmark. If `overwrite` is false and the name exists, returns
/// [`ConfigError::AlreadyExists`]. Returns `(path, replaced)`.
pub fn add(name: &str, path: &str, overwrite: bool) -> Result<(PathBuf, bool), ConfigError> {
    validate_name(name)?;
    let mut bookmarks = load()?;
    let replaced = if let Some(existing) = bookmarks.get(name) {
        if !overwrite {
            return Err(ConfigError::AlreadyExists(
                name.to_string(),
                existing.clone(),
            ));
        }
        true
    } else {
        false
    };

    let expanded = expand_path(path);
    let absolute = if expanded.exists() {
        fs::canonicalize(&expanded).unwrap_or(expanded)
    } else {
        make_absolute(&expanded)
    };

    bookmarks.insert(name.to_string(), absolute.clone());
    save(&bookmarks)?;
    Ok((absolute, replaced))
}

pub fn remove(name: &str) -> Result<PathBuf, ConfigError> {
    validate_name(name)?;
    let mut bookmarks = load()?;
    let path = bookmarks
        .remove(name)
        .ok_or_else(|| ConfigError::NotFound(name.to_string()))?;
    save(&bookmarks)?;
    Ok(path)
}

pub fn resolve(name: &str) -> Result<PathBuf, ConfigError> {
    validate_name(name)?;
    let bookmarks = load()?;
    bookmarks
        .get(name)
        .cloned()
        .ok_or_else(|| ConfigError::NotFound(name.to_string()))
}

fn make_absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}
