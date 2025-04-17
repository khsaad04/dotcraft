# Dotcraft
Yet another dotfile manager

```console
$ dotcraft --help
Dotfiles manager for unix-like operating systems 

Usage: dotcraft [OPTION] <SUBCOMMAND>

Options:
    -m, --manifest <FILE>  Path to Manifest file [default: ./Manifest.toml]
    -h, --help             Print help

Subcommands:
    sync                   Symlink files and generate templates 
    link                   Symlink files
    generate               Generate templates
```

## TODO

- [ ] Finish writing the README explaining the usage with good examples
- [x] Support different color scheme variants from [Material Design](https://m3.material.io/)
- [ ] Add a `clean` command
- [ ] Implement a lockfile
- [ ] Write my own template engine (Some day)
