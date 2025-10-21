#compdef pack

autoload -U is-at-least

_pack() {
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
'--max-depth=[Maximum depth of filesystem traversal allowed for \`pack\`]:MAXDEPTH:_default' \
'--munge=[Set the name munging policy; applies to '\''.'\'', '\''..'\'', and files with NUL and '\''/'\'' in them]:MUNGE:(filter rename)' \
'-o+[Sets the output file for saving changes (defaults to stdout)]:OUTPUT:_default' \
'--output=[Sets the output file for saving changes (defaults to stdout)]:OUTPUT:_default' \
'-t+[Specify the target format explicitly (by default, automatically inferred from filename extension)]:TARGET_FORMAT:(json toml yaml)' \
'--target=[Specify the target format explicitly (by default, automatically inferred from filename extension)]:TARGET_FORMAT:(json toml yaml)' \
'-q[Quiet mode (turns off all errors and warnings, enables \`--no-output\`)]' \
'--quiet[Quiet mode (turns off all errors and warnings, enables \`--no-output\`)]' \
'--time[Emit timing information on stderr in an '\''event,time'\'' format; time is in nanoseconds]' \
'-d[Give debug output on stderr]' \
'--debug[Give debug output on stderr]' \
'--exact[Don'\''t add newlines to the end of values that don'\''t already have them (or strip them when loading)]' \
'-P[Never follow symbolic links. This is the default behaviour. \`pack\` will ignore all symbolic links.]' \
'-L[Follow all symlinks. For safety, you can also specify a --max-depth value.]' \
'--allow-symlink-escape[Allows pack to follow symlinks outside of the directory being packed.]' \
'--no-xattr[Don'\''t use extended attributes to track metadata (see \`man xattr\`)]' \
'--keep-macos-xattr[Include ._* extended attribute/resource fork files on macOS]' \
'--no-output[Disables output of filesystem (normally on stdout)]' \
'--pretty[Pretty-print output (may increase size)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'::INPUT -- The directory to be packed:_default' \
&& ret=0
}

(( $+functions[_pack_commands] )) ||
_pack_commands() {
    local commands; commands=()
    _describe -t commands 'pack commands' commands "$@"
}

if [ "$funcstack[1]" = "_pack" ]; then
    _pack "$@"
else
    compdef _pack pack
fi
