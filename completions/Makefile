.PHONY: all

all:
	for tool in ffs pack unpack; do for sh in bash fish zsh; do cargo run --bin $$tool -- --completions $$sh >$$tool.$$sh; done; done

