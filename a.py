#!/usr/bin/env python3

import sys
import re

def matches(pat, x):
    print("RE", pat)
    return re.search(pat, x)

def assemble(strs, subs, var):
    for str_, sub_ in zip(strs, subs):
        yield str_
        yield var.get(sub_, "")
    yield strs[-1]

def main():
    if len(sys.argv) <= 1:
        return 1

    pairs = []
    for i in range(1, len(sys.argv), 2):
        pat = sys.argv[i]
        if i + 1 == len(sys.argv):
            sub = "{@}"
        else:
            sub = sys.argv[i + 1]
        pairs.append((pat, sub))

    patsubs = []
    for pat, sub in pairs:
        strs = []
        subs = []

        last = 0
        for v in re.finditer("\{[^}]*\}", sub):
            strs.append(sub[last:v.start()])
            subs.append("P" + sub[v.start()+1:v.end()-1])
            last = v.end()
        strs.append(sub[last:])

        tree = parse("{" + pat + "}", 0, 0)[1][1]
        pat = "".join(tree_to_regex(tree, False))
        patsubs.append((pat, strs, subs))


    for x in sys.stdin:
        x = x.rstrip()

        for pat, strs, subs in patsubs:
            if m := matches(pat, x):
                var = m.groupdict()
                var["P^"] = x[:m.start()]
                var["P$"] = x[m.end():]
                var["P%"] = x[m.start():m.end()]
                var["P@"] = x[:]

                y = "".join(assemble(strs, subs, var))

                sys.stdout.write(y)
                sys.stdout.write("\n")
                sys.stdout.flush()
                break


def parse(x, i, lvl):
    escape = False
    tree = []

    buf = []
    while i < len(x):
        if escape and i == len(x) - 1:
            buf.append("\\")
            escape = False
        elif escape:
            if x[i] in "{}":
                buf.append(x[i])
            elif x[i] == "\\":
                buf.append("\\")
            else:
                buf.append("\\")
                buf.append(x[i])
            i += 1
            escape = False
            continue

        if x[i] == "\\":
            i += 1
            escape = True
            continue


        if x[i] == "{":
            tree.append("".join(buf))
            i, subtree = parse(x, i + 1, lvl + 1)
            tree.append(subtree)
            buf = []
        elif x[i] == "}":
            tree.append("".join(buf))
            if lvl == 0 and i < len(x) - 1:
                raise ValueError("Parse error: closing }")
            return i, tree
        else:
            buf.append(x[i])

        i += 1

    if lvl > 0:
        raise ValueError("Parse error: unclosed {")
    return i, tree


def tree_to_regex(tree, is_group):
    if is_group:
        if tree[0] == "":
            group, rx = "EEE", "[a-zA-Z0-9]+"
        elif ":" in tree[0]:
            group, rx = tree[0].split(":", 1)
        else:
            group = tree[0]
            if group[0].isnumeric():
                rx = "[0-9]+"
            elif group[0].isalpha():
                rx = "[a-zA-Z0-9]+"
                rx = "[^/]+"
            else:
                raise ValueError("Group name must be alphanumeric only")
            group, rx = tree[0], rx

        yield f"(?P<P{group}>{rx}"

        yield from tree_to_regex(tree[1:], False)
        yield ")"
        return

    for x in tree:
        if isinstance(x, str):
            yield x
        else:
            yield from tree_to_regex(x, True)

def test_parse():
    pat = sys.argv[1]
    x = parse("{" + pat + "}", 0, 0)[1][1]
    print(x)
    y = tree_to_regex(x, False)
    print("".join(y))

if __name__ == "__main__":
    #test_parse()
    main()
