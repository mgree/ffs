#!/usr/bin/env python3

from sys import argv, stderr
import json

counter = 0
letters = list("abcdefghijklmnopqrstuvwxyz")

def fresh_name():
    global counter

    num = counter
    counter = counter + 1

    name = ""    
    while num > 0 or name == "":
        name += letters[num % 26]
        num //= 26

    return name

def object(field,value):
    o = dict()
    o[field] = value
    return o

def deep(kind, depth):
    if kind == "list":
        if depth <= 0:
            return list()
        
        f = lambda v: [v]
    elif kind == "named":
        if depth <= 0:
            return dict()

        def f(v):
            o = dict()
            o[fresh_name()] = v
            return o
    else:
        raise(ValueError("Unknown kind '{}'".format(kind)))
    
    v = None
    for i in range(0,depth):
        v = f(v)

    return v

def wide(kind, width):
    if kind == "list":
        v = list()
        
        def f(v):
            v.append(None)
            return v
    elif kind == "named":
        v = dict()

        def f(v):
            v[fresh_name()] = None
            return v
    else:
        raise(ValueError("Unknown kind '{}'".format(kind)))

    for i in range(0,width):
        v = f(v)

    return v

def usage():
    print("Usage: {} [list|named] [wide|deep] [size]", file=stderr)
    exit(2)

if __name__ == "__main__":
    if len(argv) != 4:
        usage()

    kind,approach,size = argv[1:]

    if kind.lower().strip() not in ["named", "list"]:
        print("Unknown kind '{}' (expected 'list' or 'named')".format(kind), file=stderr)
        usage()
    else:
        kind = kind.lower().strip()
        
    if approach.lower().strip() == "wide":
        f = wide
    elif approach.lower().strip() == "deep":
        f = deep
    else:
        print("Unknown approach '{}' (expected 'wide' or 'deep')".format(kind), file=stderr)
        usage()

    try:
        size = int(size)
    except:
        print("Unknown size '{}', expected non-negative number".format(size), file=stderr)
        usage()

    print(json.dumps(f(kind, size), separators=(',', ':')))
