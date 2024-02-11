# `uninstall`

Uninstalls rye again.  Note that this leaves a trace
`.rye` folder behind with an empty `env` file.  You also
need to remove the sourcing of that script from your
`.profile` file.

## Example

Uninstall rye without asking:

```
$ rye self uninstall --yes
```

## Arguments

_no arguments_
    
## Options

* `-y, --yes`: Do not prompt and uninstall.

* `-h, --help`: Print help (see a summary with '-h')
