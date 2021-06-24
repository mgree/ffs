---
title: "ffs: the file fileystem"
author: "[Michael Greenberg](http://mgree.github.io)"
---

# ffs: the file filesystem

The Unix shell is a powerful tool, and the Unix ecosystem provides an
incredible array of tools for working with strings. But the shell
really only knows how to work with one data *structure*: the
filesystem. Modern systems use all kinds of
[*semi-structured*](https://en.m.wikipedia.org/wiki/Semi-structured_data)
data, like JSON or YAML. These semi-structured formats are essentially
trees, and string processing is a bad match---editing JSON with `sed`
is not a very good idea!

# Examples

# Getting ffs

You can get ffs from the [ffs GitHub repo](https://github.com/mgree/ffs).

# Related tools

Tools like [`jq`](https://stedolan.github.io/jq/) and
[`gron`](https://github.com/tomnomnom/gron) are meant to help you work
with JSON on the command line. They're great tools! Why might ffs be
the right choice for you?

  - ffs supports multiple formats.

  - ffs lets you edit using familiar shell tools.
  
  - ffs doesn't involve learning a new language.
