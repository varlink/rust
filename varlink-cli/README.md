```
$ cargo -q run -- --help
varlink 3.0.0

USAGE:
    varlink [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       
            Prints help information

    -V, --version    
            Prints version information


OPTIONS:
    -A, --activate <COMMAND>    
            Service to socket-activate and connect to. The temporary UNIX socket address is exported as
            $VARLINK_ADDRESS.
    -b, --bridge <COMMAND>      
            Command to execute and connect to

        --color <color>         
            colorize output [default: auto]  [possible values: on, off, auto]

    -R, --resolver <ADDRESS>    
            address of the resolver [default: unix:/run/org.varlink.resolver]


SUBCOMMANDS:
    bridge         Bridge varlink messages from stdio to services on this machine
    call           Call a method
    completions    Generates completion scripts for your shell
    format         Format a varlink service file
    help           Print interface description or service information
    info           Print information about a service
    resolve        Resolve an interface name to a varlink address
```

[![asciicast](https://asciinema.org/a/214448.svg)](https://asciinema.org/a/214448)
