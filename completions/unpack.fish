complete -c unpack -l completions -d 'Generate shell completions (and exit)' -r -f -a "bash\t''
elvish\t''
fish\t''
powershell\t''
zsh\t''"
complete -c unpack -l munge -d 'Set the name munging policy; applies to \'.\', \'..\', and files with NUL and \'/\' in them' -r -f -a "filter\t''
rename\t''"
complete -c unpack -s t -l type -d 'Specify the format type explicitly (by default, automatically inferred from filename extension)' -r -f -a "json\t''
toml\t''
yaml\t''"
complete -c unpack -s i -l into -d 'Sets the directory in which to unpack the file; will be inferred when using a file, but must be specified when running on stdin' -r
complete -c unpack -s q -l quiet -d 'Quiet mode (turns off all errors and warnings, enables `--no-output`)'
complete -c unpack -l time -d 'Emit timing information on stderr in an \'event,time\' format; time is in nanoseconds'
complete -c unpack -s d -l debug -d 'Give debug output on stderr'
complete -c unpack -l exact -d 'Don\'t add newlines to the end of values that don\'t already have them (or strip them when loading)'
complete -c unpack -l no-xattr -d 'Don\'t use extended attributes to track metadata (see `man xattr`)'
complete -c unpack -l unpadded -d 'Don\'t pad the numeric names of list elements with zeroes; will not sort properly'
complete -c unpack -s h -l help -d 'Print help'
complete -c unpack -s V -l version -d 'Print version'
