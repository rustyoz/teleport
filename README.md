# tp — teleport

`tp` is a small directory bookmark tool. Think `cd`, but you jump by name.

Bookmarks live in `~/.tp/tp.conf`.

## Install

```bash
cargo install --path .
```

Wire up shell integration so `tp NAME` actually changes directory (a child process cannot change the parent shell's cwd):

```bash
# bash
echo 'eval "$(tp --init)"' >> ~/.bashrc
source ~/.bashrc

# zsh
echo 'eval "$(tp --init)"' >> ~/.zshrc
source ~/.zshrc
```

## Usage

```bash
tp --add work ~/git/work
tp --add docs ~/Documents

tp --list
# docs  /home/you/Documents
# work  /home/you/git/work

tp work          # cd to that path
tp --remove work
```

Without arguments, `tp` lists bookmarks.
