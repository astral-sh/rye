# `completion`

Generates a completion script for a shell

## Example

Generate a completion script for zsh and load it:

```
$ eval "$(rye self completion -s zsh)"
```

## Arguments

_no arguments_
    
## Options

* `-s, --shell <SHELL>`: The shell to generate a completion script for (defaults to 'bash')

    [possible values: `bash`, `elvish`, `fish`, `powershell`, `zsh`, `nushell`]

* `-h, --help`: Print help (see a summary with '-h')
