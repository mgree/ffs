_ffs() {
    local i cur prev opts cmds
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    for i in ${COMP_WORDS[@]}
    do
        case "${i}" in
            ffs)
                cmd="ffs"
                ;;
            
            *)
                ;;
        esac
    done

    case "${cmd}" in
        ffs)
            opts=" -q -d -i -h -V -u -g -o -s -t -m  --quiet --time --debug --eager --exact --no-xattr --keep-macos-xattr --unpadded --readonly --no-output --in-place --pretty --help --version --completions --uid --gid --mode --dirmode --munge --output --source --target --mount --new  <INPUT> "
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                
                --completions)
                    COMPREPLY=($(compgen -W "bash fish zsh" -- "${cur}"))
                    return 0
                    ;;
                --uid)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -u)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --gid)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -g)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --mode)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dirmode)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --munge)
                    COMPREPLY=($(compgen -W "filter rename" -- "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --source)
                    COMPREPLY=($(compgen -W "json toml yaml" -- "${cur}"))
                    return 0
                    ;;
                    -s)
                    COMPREPLY=($(compgen -W "json toml yaml" -- "${cur}"))
                    return 0
                    ;;
                --target)
                    COMPREPLY=($(compgen -W "json toml yaml" -- "${cur}"))
                    return 0
                    ;;
                    -t)
                    COMPREPLY=($(compgen -W "json toml yaml" -- "${cur}"))
                    return 0
                    ;;
                --mount)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                    -m)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --new)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        
    esac
}

complete -F _ffs -o bashdefault -o default ffs
