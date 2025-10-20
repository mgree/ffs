complete -c ffs -l completions -d 'Generate shell completions (and exit)' -r -f -a "bash\t''
elvish\t''
fish\t''
powershell\t''
zsh\t''"
complete -c ffs -s u -l uid -d 'Sets the user id of the generated filesystem (defaults to current effective user id)' -r
complete -c ffs -s g -l gid -d 'Sets the group id of the generated filesystem (defaults to current effective group id)' -r
complete -c ffs -l mode -d 'Sets the default mode of files (parsed as octal)' -r
complete -c ffs -l dirmode -d 'Sets the default mode of directories (parsed as octal; if unspecified, directories will have FILEMODE with execute bits set when read bits are set)' -r
complete -c ffs -l munge -d 'Set the name munging policy; applies to \'.\', \'..\', and files with NUL and \'/\' in them' -r -f -a "filter\t''
rename\t''"
complete -c ffs -s o -l output -d 'Sets the output file for saving changes (defaults to stdout)' -r
complete -c ffs -s s -l source -d 'Specify the source format explicitly (by default, automatically inferred from filename extension)' -r -f -a "json\t''
toml\t''
yaml\t''"
complete -c ffs -s t -l target -d 'Specify the target format explicitly (by default, automatically inferred from filename extension)' -r -f -a "json\t''
toml\t''
yaml\t''"
complete -c ffs -s m -l mount -d 'Sets the mountpoint; will be inferred when using a file, but must be specified when running on stdin' -r
complete -c ffs -l new -d 'Mounts an empty filesystem, inferring a mountpoint and output format' -r
complete -c ffs -s q -l quiet -d 'Quiet mode (turns off all errors and warnings, enables `--no-output`)'
complete -c ffs -l time -d 'Emit timing information on stderr in an \'event,time\' format; time is in nanoseconds'
complete -c ffs -s d -l debug -d 'Give debug output on stderr'
complete -c ffs -l eager -d 'Eagerly load data on startup (data is lazily loaded by default)'
complete -c ffs -l exact -d 'Don\'t add newlines to the end of values that don\'t already have them (or strip them when loading)'
complete -c ffs -l no-xattr -d 'Don\'t use extended attributes to track metadata (see `man xattr`)'
complete -c ffs -l keep-macos-xattr -d 'Include ._* extended attribute/resource fork files on macOS'
complete -c ffs -l unpadded -d 'Don\'t pad the numeric names of list elements with zeroes; will not sort properly'
complete -c ffs -l readonly -d 'Mounted filesystem will be readonly'
complete -c ffs -l no-output -d 'Disables output of filesystem (normally on stdout)'
complete -c ffs -s i -l in-place -d 'Writes the output back over the input file'
complete -c ffs -l pretty -d 'Pretty-print output (may increase size)'
complete -c ffs -s h -l help -d 'Print help'
complete -c ffs -s V -l version -d 'Print version'
