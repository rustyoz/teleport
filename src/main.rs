mod config;

use std::env;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("tp: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        list_bookmarks()?;
        return Ok(());
    };

    match first.as_str() {
        "-h" | "--help" => {
            print_help();
            Ok(())
        }
        "-V" | "--version" => {
            println!("tp {VERSION}");
            Ok(())
        }
        "--init" => {
            print!("{}", shell_init());
            Ok(())
        }
        "--list" => list_bookmarks().map_err(Into::into),
        "--add" => {
            let name = args.next().ok_or("usage: tp --add NAME PATH")?;
            let path = args.next().ok_or("usage: tp --add NAME PATH")?;
            if args.next().is_some() {
                return Err("usage: tp --add NAME PATH".into());
            }
            add_bookmark(&name, &path)
        }
        "--remove" => {
            let name = args
                .next()
                .ok_or("usage: tp --remove NAME")?;
            if args.next().is_some() {
                return Err("usage: tp --remove NAME".into());
            }
            let path = config::remove(&name)?;
            eprintln!("removed '{name}' -> {}", path.display());
            Ok(())
        }
        name if name.starts_with('-') => {
            Err(format!("unknown option '{name}'\ntry 'tp --help'").into())
        }
        name => {
            if args.next().is_some() {
                return Err("usage: tp NAME".into());
            }
            let path = config::resolve(name)?;
            println!("{}", path.display());
            Ok(())
        }
    }
}

fn add_bookmark(name: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    match config::add(name, path, false) {
        Ok((absolute, _)) => {
            eprintln!("added '{name}' -> {}", absolute.display());
            Ok(())
        }
        Err(config::ConfigError::AlreadyExists(_, existing)) => {
            let prompt = format!(
                "'{name}' already points to {}. overwrite?",
                existing.display()
            );
            if !confirm(&prompt)? {
                eprintln!("cancelled");
                return Ok(());
            }
            let (absolute, _) = config::add(name, path, true)?;
            eprintln!("updated '{name}' -> {}", absolute.display());
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

fn confirm(prompt: &str) -> io::Result<bool> {
    let mut tty = match OpenOptions::new().read(true).write(true).open("/dev/tty") {
        Ok(tty) => tty,
        Err(_) => {
            eprintln!("tp: no tty to confirm overwrite; cancelled");
            return Ok(false);
        }
    };
    write!(tty, "{prompt} [y/N] ")?;
    tty.flush()?;

    let mut line = String::new();
    BufReader::new(tty).read_line(&mut line)?;
    let answer = line.trim();
    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}

fn list_bookmarks() -> Result<(), config::ConfigError> {
    let bookmarks = config::load()?;
    if bookmarks.is_empty() {
        eprintln!("no bookmarks yet — add one with: tp --add NAME PATH");
        return Ok(());
    }

    let width = bookmarks.keys().map(|k| k.len()).max().unwrap_or(0);
    for (name, path) in &bookmarks {
        println!("{name:<width$}  {}", path.display());
    }
    Ok(())
}

fn print_help() {
    print!(
        "\
tp — teleport to named directories

Usage:
  tp NAME                 print bookmarked path (shell wrapper cds there)
  tp --add NAME PATH      add a bookmark (prompts before overwrite)
  tp --list               list bookmarks
  tp --remove NAME        remove a bookmark
  tp --init               print shell integration
  tp -h, --help           show this help
  tp -V, --version        show version

Bookmarks are stored in ~/.tp/tp.conf.

Because a program cannot change its parent shell's working directory, add:
  eval \"$(tp --init)\"
to your shell rc file so `tp NAME` cds into the bookmarked path.
"
    );
}

fn shell_init() -> &'static str {
    r#"# tp shell integration — add to ~/.bashrc or ~/.zshrc:
#   eval "$(tp --init)"
tp() {
  local cmd
  cmd="$(command -v tp 2>/dev/null)" || cmd=tp
  case "${1-}" in
    --add|--list|--remove|--init|-h|--help|--version|-V|"")
      command "$cmd" "$@"
      ;;
    -*)
      command "$cmd" "$@"
      ;;
    *)
      local dest
      dest="$(command "$cmd" "$1")" || return $?
      if [ -n "$dest" ]; then
        cd -- "$dest" || return $?
      fi
      ;;
  esac
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_init_mentions_cd() {
        assert!(shell_init().contains("builtin cd"));
    }
}
