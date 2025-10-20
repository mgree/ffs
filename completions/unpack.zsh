#compdef unpack

autoload -U is-at-least

_unpack() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'--completions=[Generate shell completions (and exit)]:SHELL:(bash elvish fish powershell zsh)' \
'--munge=[Set the name munging policy; applies to '\''.'\'', '\''..'\'', and files with NUL and '\''/'\'' in them]:MUNGE:(filter rename)' \
'-t+[Specify the format type explicitly (by default, automatically inferred from filename extension)]:TYPE:(json toml yaml)' \
'--type=[Specify the format type explicitly (by default, automatically inferred from filename extension)]:TYPE:(json toml yaml)' \
'-i+[Sets the directory in which to unpack the file; will be inferred when using a file, but must be specified when running on stdin]:INTO:_default' \
'--into=[Sets the directory in which to unpack the file; will be inferred when using a file, but must be specified when running on stdin]:INTO:_default' \
'-q[Quiet mode (turns off all errors and warnings, enables \`--no-output\`)]' \
'--quiet[Quiet mode (turns off all errors and warnings, enables \`--no-output\`)]' \
'--time[Emit timing information on stderr in an '\''event,time'\'' format; time is in nanoseconds]' \
'-d[Give debug output on stderr]' \
'--debug[Give debug output on stderr]' \
'--exact[Don'\''t add newlines to the end of values that don'\''t already have them (or strip them when loading)]' \
'--no-xattr[Don'\''t use extended attributes to track metadata (see \`man xattr\`)]' \
'--unpadded[Don'\''t pad the numeric names of list elements with zeroes; will not sort properly]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'::INPUT -- Sets the input file ('\''-'\'' means STDIN):_default' \
&& ret=0
}

(( $+functions[_unpack_commands] )) ||
_unpack_commands() {
    local commands; commands=()
    _describe -t commands 'unpack commands' commands "$@"
}

if [ "$funcstack[1]" = "_unpack" ]; then
    _unpack "$@"
else
    compdef _unpack unpack
fi
