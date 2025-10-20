complete -c pack -l completions -d 'Generate shell completions (and exit)' -r -f -a "bash\t''
elvish\t''
fish\t''
powershell\t''
zsh\t''"
complete -c pack -l max-depth -d 'Maximum depth of filesystem traversal allowed for `pack`' -r
complete -c pack -l munge -d 'Set the name munging policy; applies to \'.\', \'..\', and files with NUL and \'/\' in them' -r -f -a "filter\t''
rename\t''"
complete -c pack -s o -l output -d 'Sets the output file for saving changes (defaults to stdout)' -r
complete -c pack -s t -l target -d 'Specify the target format explicitly (by default, automatically inferred from filename extension)' -r -f -a "json\t''
toml\t''
yaml\t''"
complete -c pack -s q -l quiet -d 'Quiet mode (turns off all errors and warnings, enables `--no-output`)'
complete -c pack -l time -d 'Emit timing information on stderr in an \'event,time\' format; time is in nanoseconds'
complete -c pack -s d -l debug -d 'Give debug output on stderr'
complete -c pack -l exact -d 'Don\'t add newlines to the end of values that don\'t already have them (or strip them when loading)'
complete -c pack -s P -d 'Never follow symbolic links. This is the default behaviour. `pack` will ignore all symbolic links.'
complete -c pack -s L -d 'Follow all symlinks. For safety, you can also specify a --max-depth value.'
complete -c pack -l allow-symlink-escape -d 'Allows pack to follow symlinks outside of the directory being packed.'
complete -c pack -l no-xattr -d 'Don\'t use extended attributes to track metadata (see `man xattr`)'
complete -c pack -l keep-macos-xattr -d 'Include ._* extended attribute/resource fork files on macOS'
complete -c pack -l no-output -d 'Disables output of filesystem (normally on stdout)'
complete -c pack -l pretty -d 'Pretty-print output (may increase size)'
complete -c pack -s h -l help -d 'Print help'
complete -c pack -s V -l version -d 'Print version'
