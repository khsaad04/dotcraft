# Dotman
Yet another dotfile manager

```console
$ dotman --help
Yet another dotfile manager

Usage: dotman [OPTIONS] <SUBCOMMAND>

Options:
    -m, --manifest <PATH>  Path to Manifest.toml [default: ./Manifest.toml]
    -h, --help             Print help

Subcommands:
    sync      Symlink files and generate templates 
    link      Symlink files
    generate  Generate templates
```

## TODO

- [ ] Finish writing the README explaining the usage with good examples
- [ ] Support different color scheme variants as per the [Material You](https://m3.material.io/) specs
- [ ] Implement a lockfile
    - [ ] Serialize the lockfile with checksums of templates and wallpaper
    - [ ] Detect if templates and color palette need regenerating
    - [ ] Save the color palette to lockfile to avoid recomputation
- [ ] Improve the cli UX
    - [ ] Ability to quickly generate some templates without needing `Manifest.toml`
    - [ ] Ability to display the colors in the terminal
    - [ ] Command to unlink files that were linked
    - [ ] Better logging
- [ ] Write my own template engine (Some day)
