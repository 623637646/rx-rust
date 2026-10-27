"""Fails when the code compiles differently on a configuration CI does not build.

CI checks the code in every configuration it compiles differently in: every combination of the
features, the dev and the release profile, ... (see `ci/check.sh`). A new `cfg` key, such as a
`cfg(target_os = "windows")`, adds a configuration that none of those runs builds, and that no
tool would then check. This script collects every `cfg` key the code uses — in `cfg(...)`,
`cfg_attr(...)` and `cfg!(...)`, in the Rust sources, the doc examples and the README examples —
and fails on one that `ci/cfg-dimensions.toml` does not list, or on a value it does not allow.

It also checks the values of `feature` against the manifest: rustc's `unexpected_cfgs` does that
for the code, but not for a doc example.

Run it from the repository root: `python3 ci/cfg_coverage.py`.
"""

import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "ci" / "cfg-dimensions.toml"

TOKEN = re.compile(r'\s*([A-Za-z_][A-Za-z0-9_]*|"(?:[^"\\]|\\.)*"|[(),=]|.)')
START = re.compile(r"(?<![A-Za-z0-9_])(cfg_attr|cfg!?)\s*\(")
COMBINATORS = {"all", "any", "not"}


def code_of(path):
    """The text of `path` that can hold a `cfg`, one string per source line, so that a finding
    keeps its line number: the code of a Rust file, its comments left out except for the lines of
    a doc example, and the Rust code blocks of a Markdown file."""
    lines = path.read_text().splitlines()
    if path.suffix == ".md":
        kept, in_rust = [], False
        for line in lines:
            if line.startswith("```"):
                in_rust = not in_rust and line[3:].strip().split(",")[0] in ("", "rust")
                kept.append("")
            else:
                kept.append(line if in_rust else "")
        return kept
    kept = []
    for line in lines:
        code, _, comment = line.partition("//")
        # A doc comment of an example: `/// #[cfg(..)]`, or hidden, `/// # #[cfg(..)]`.
        example = re.match(r"[/!]\s*(?:#\s+)?(#!?\[.*)", comment)
        kept.append(code + (" " + example.group(1) if example else ""))
    return kept


def tokens(text, start):
    """The tokens of `text` from `start` up to the parenthesis that closes the one before it."""
    depth, result, position = 1, [], start
    while position < len(text) and depth:
        match = TOKEN.match(text, position)
        token = match.group(1)
        position = match.end()
        depth += {"(": 1, ")": -1}.get(token, 0)
        if depth:
            result.append(token)
    return result


def predicate(stream):
    """Parses one predicate off the front of `stream`, a list of tokens, and yields its leaves as
    `(key, value)` pairs, `value` being `None` for a bare key such as `test`."""
    key = stream.pop(0)
    if stream and stream[0] == "(":
        stream.pop(0)
        while stream and stream[0] != ")":
            yield from predicate(stream)
            if stream and stream[0] == ",":
                stream.pop(0)
        if stream:
            stream.pop(0)
        if key not in COMBINATORS:
            yield key, None
    elif stream and stream[0] == "=":
        stream.pop(0)
        yield key, stream.pop(0).strip('"')
    else:
        yield key, None


def uses(path):
    """Every `(line, key, value)` that `path` puts in a `cfg`."""
    lines = code_of(path)
    text = "\n".join(lines)
    for match in START.finditer(text):
        stream = tokens(text, match.end())
        line = text.count("\n", 0, match.start()) + 1
        # `cfg_attr(predicate, attributes)`: only the first argument is a predicate.
        for key, value in predicate(stream) if stream else ():
            yield line, key, value


def features():
    """The features a `cfg(feature = ..)` can name: the declared ones, and the implicit feature of
    every optional dependency that no `dep:` refers to."""
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    declared = set(manifest.get("features", {}))
    referred = {
        item[len("dep:"):]
        for items in manifest.get("features", {}).values()
        for item in items
        if item.startswith("dep:")
    }
    optional = {
        name
        for name, spec in manifest.get("dependencies", {}).items()
        if isinstance(spec, dict) and spec.get("optional")
    }
    return declared | (optional - referred)


def main():
    registry = tomllib.loads(REGISTRY.read_text())
    known_features = features()
    files = sorted(
        path
        for pattern in ("src/**/*.rs", "tests/**/*.rs", "examples/**/*.rs", "benches/**/*.rs")
        for path in ROOT.glob(pattern)
    ) + [ROOT / "README.md"]
    problems, seen = [], set()
    for path in files:
        for line, key, value in uses(path):
            where = f"{path.relative_to(ROOT)}:{line}"
            seen.add(key)
            entry = registry.get(key)
            if entry is None:
                problems.append(
                    f"{where}: `cfg({key})` is a configuration CI does not build; extend the CI "
                    f"matrix to build both sides of it, then list it in {REGISTRY.name}"
                )
            elif key == "feature" and value not in known_features:
                problems.append(f"{where}: `feature = \"{value}\"` is not a feature of Cargo.toml")
            elif "values" in entry and value not in entry["values"]:
                problems.append(
                    f"{where}: `cfg({key} = \"{value}\")` is a value CI does not build; extend the "
                    f"CI matrix, then add it to the values of `{key}` in {REGISTRY.name}"
                )
    for key in sorted(set(registry) - seen):
        problems.append(
            f"{REGISTRY.name}: `{key}` is no longer used by any `cfg`; remove it, and what CI runs "
            "only for it"
        )
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        sys.exit(1)
    print(f"every cfg key is covered by CI: {', '.join(sorted(seen))}")


if __name__ == "__main__":
    main()
