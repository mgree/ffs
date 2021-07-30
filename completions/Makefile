.PHONY: all

all:
	for x in bash fish zsh; do cargo run -- --completions $$x >ffs.$$x; done
