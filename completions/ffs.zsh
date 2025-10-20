#compdef ffs

autoload -U is-at-least

_ffs() {
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
'-u+[Sets the user id of the generated filesystem (defaults to current effective user id)]:UID:_default' \
'--uid=[Sets the user id of the generated filesystem (defaults to current effective user id)]:UID:_default' \
'-g+[Sets the group id of the generated filesystem (defaults to current effective group id)]:GID:_default' \
'--gid=[Sets the group id of the generated filesystem (defaults to current effective group id)]:GID:_default' \
'--mode=[Sets the default mode of files (parsed as octal)]:FILEMODE:_default' \
'--dirmode=[Sets the default mode of directories (parsed as octal; if unspecified, directories will have FILEMODE with execute bits set when read bits are set)]:DIRMODE:_default' \
'--munge=[Set the name munging policy; applies to '\''.'\'', '\''..'\'', and files with NUL and '\''/'\'' in them]:MUNGE:(filter rename)' \
'-o+[Sets the output file for saving changes (defaults to stdout)]:OUTPUT:_default' \
'--output=[Sets the output file for saving changes (defaults to stdout)]:OUTPUT:_default' \
'-s+[Specify the source format explicitly (by default, automatically inferred from filename extension)]:SOURCE_FORMAT:(json toml yaml)' \
'--source=[Specify the source format explicitly (by default, automatically inferred from filename extension)]:SOURCE_FORMAT:(json toml yaml)' \
'-t+[Specify the target format explicitly (by default, automatically inferred from filename extension)]:TARGET_FORMAT:(json toml yaml)' \
'--target=[Specify the target format explicitly (by default, automatically inferred from filename extension)]:TARGET_FORMAT:(json toml yaml)' \
'-m+[Sets the mountpoint; will be inferred when using a file, but must be specified when running on stdin]:MOUNT:_default' \
'--mount=[Sets the mountpoint; will be inferred when using a file, but must be specified when running on stdin]:MOUNT:_default' \
'(-i --in-place -s --source -o --output)--new=[Mounts an empty filesystem, inferring a mountpoint and output format]:NEW:_default' \
'-q[Quiet mode (turns off all errors and warnings, enables \`--no-output\`)]' \
'--quiet[Quiet mode (turns off all errors and warnings, enables \`--no-output\`)]' \
'--time[Emit timing information on stderr in an '\''event,time'\'' format; time is in nanoseconds]' \
'-d[Give debug output on stderr]' \
'--debug[Give debug output on stderr]' \
'--eager[Eagerly load data on startup (data is lazily loaded by default)]' \
'--exact[Don'\''t add newlines to the end of values that don'\''t already have them (or strip them when loading)]' \
'--no-xattr[Don'\''t use extended attributes to track metadata (see \`man xattr\`)]' \
'--keep-macos-xattr[Include ._* extended attribute/resource fork files on macOS]' \
'--unpadded[Don'\''t pad the numeric names of list elements with zeroes; will not sort properly]' \
'--readonly[Mounted filesystem will be readonly]' \
'--no-output[Disables output of filesystem (normally on stdout)]' \
'-i[Writes the output back over the input file]' \
'--in-place[Writes the output back over the input file]' \
'--pretty[Pretty-print output (may increase size)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'::INPUT -- Sets the input file ('\''-'\'' means STDIN):_default' \
&& ret=0
}

(( $+functions[_ffs_commands] )) ||
_ffs_commands() {
    local commands; commands=()
    _describe -t commands 'ffs commands' commands "$@"
}

if [ "$funcstack[1]" = "_ffs" ]; then
    _ffs "$@"
else
    compdef _ffs ffs
fi
