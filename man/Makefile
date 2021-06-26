.PHONY: clean

ffs.1: ffs.1.md
	pandoc --standalone --from markdown-smart --to man $< -o $@

clean:
	rm ffs.1
