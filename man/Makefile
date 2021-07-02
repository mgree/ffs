.PHONY: clean

ffs.1: ../docs/ffs.1.md
	pandoc --standalone --from markdown-smart --to man $< -o $@

clean:
	rm ffs.1
