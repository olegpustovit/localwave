#!/usr/bin/env python3
"""Split rustfmt-formatted top-level Rust items: split-rs.py SRC MANIFEST.toml.

Manifest sections name modules; each contains items = ["Name", "function", ...].
Matching impl blocks follow their type. Existing destination files are never overwritten.
Conservation checks cover original byte spans before visibility/import/module syntax changes.
Requires Python 3.11+ for the standard-library TOML reader.
"""
import argparse
from collections import Counter
from pathlib import Path
import re
import tomllib


TOKEN = re.compile(
    r'//[^\n]*|/\*|(?:br|cr|r)(#*)"|(?:b|c)?"(?:\\.|[^"\\])*"'
    r"|b?'(?:\\.|[^'\\\n])'|[A-Za-z_]\w*|[^\s]",
    re.DOTALL,
)
DECL = re.compile(r"\b(fn|struct|enum|trait|type|const|static|mod|use|extern|macro_rules)\b")
IDENT = re.compile(r"[A-Za-z_][A-Za-z_0-9]*\Z")


def tokens(source):
    """Skip comments and keep literals opaque, including raw strings/nested comments."""
    position = 0
    while match := TOKEN.search(source, position):
        value = match[0]
        position = match.end()
        if value.startswith("//"):
            continue
        if value == "/*":
            depth = 1
            while depth:
                next_comment = re.search(r"/\*|\*/", source[position:])
                if next_comment is None:
                    raise ValueError("unterminated block comment")
                depth += 1 if next_comment[0] == "/*" else -1
                position += next_comment.end()
            continue
        if match[1] is not None:
            end = source.find('"' + match[1], position)
            if end < 0:
                raise ValueError("unterminated raw string")
            position = end + 1 + len(match[1])
            value = '"literal"'
        yield value, match.start(), position


def items(source):
    starts = []
    stack = []
    attribute = None
    for value, start, _ in tokens(source):
        column_zero = start == 0 or source[start - 1] == "\n"
        if not stack and column_zero:
            if source.startswith("#[", start):
                if attribute is None:
                    attribute = start
            elif value not in ("#", "]", ";", "}"):
                begin = attribute if attribute is not None else start
                # Column-zero outer docs/comments belong to the following item.
                while begin > 0:
                    previous = source.rfind("\n", 0, begin - 1) + 1
                    line = source[previous:begin].rstrip("\n")
                    if line.strip() and (not line.startswith("//") or line.startswith("//!")):
                        break
                    begin = previous
                starts.append((begin, start))
                attribute = None
        if value in ("{", "[", "("):
            stack.append(value)
        elif value in ("}", "]", ")"):
            if not stack or stack.pop() != {"}": "{", "]": "[", ")": "("}[value]:
                raise ValueError("unbalanced Rust delimiters")
    if stack or attribute is not None:
        raise ValueError("incomplete Rust item")
    result = []
    for index, (begin, declaration) in enumerate(starts):
        end = starts[index + 1][0] if index + 1 < len(starts) else len(source)
        block = source[begin:end]
        header = source[declaration:end]
        if re.match(r"(?:unsafe\s+)?impl\b", header):
            header = header.split("{", 1)[0].split("where", 1)[0]
            while re.search(r"<[^<>]*>", header):
                header = re.sub(r"<[^<>]*>", "", header)
            target = re.split(r"\bfor\b", header)[-1]
            target = re.sub(r"^(?:unsafe\s+)?impl\s*", "", target).strip()
            if not re.fullmatch(r"(?:[A-Za-z_]\w*::)*[A-Za-z_]\w*", target):
                raise ValueError(f"unsupported impl target: {target}")
            kind, name = "impl", target.split("::")[-1]
        else:
            function = re.match(
                r'(?:pub(?:\([^)]*\))?\s+)?(?:(?:async|unsafe|const)\s+)*'
                r'(?:extern\s+"[^"]*"\s+)?fn\s+([A-Za-z_]\w*)', header,
            )
            match = DECL.search(header)
            if match is None:
                raise ValueError(f"unsupported item: {header.splitlines()[0]}")
            if function:
                kind, name = "fn", function[1]
            else:
                kind = match[1]
                name_match = re.match(r"\s*!?\s*([A-Za-z_]\w*)", header[match.end():])
                name = name_match[1] if name_match else ""
        result.append((name, kind, block, declaration - begin))
    prefix = source[:starts[0][0]] if starts else source
    return prefix, result


def check_conservation(original, remaining, moved):
    if Counter(original) != Counter(remaining + moved):
        raise ValueError("item conservation failed; no files written")


def split(source_path, manifest_path):
    source_path = Path(source_path)
    source = source_path.read_bytes().decode("utf-8")
    manifest = tomllib.loads(Path(manifest_path).read_text())
    prefix, original = items(source)
    if prefix + "".join(item[2] for item in original) != source:
        raise ValueError("item spans do not cover the original bytes")
    assignments = {}
    for module, config in manifest.items():
        if not IDENT.fullmatch(module) or not isinstance(config, dict) or set(config) != {"items"}:
            raise ValueError(f"invalid module section: {module}")
        names = config["items"]
        if not isinstance(names, list) or not names:
            raise ValueError(f"{module}: items must be a nonempty array")
        for name in names:
            if not isinstance(name, str) or not IDENT.fullmatch(name) or name in assignments:
                raise ValueError(f"invalid or duplicate manifest name: {name}")
            assignments[name] = module
    if not assignments:
        raise ValueError("empty manifest")
    counts = Counter(name for name, kind, _, _ in original if kind != "impl")
    for name in assignments:
        if counts[name] != 1:
            raise ValueError(f"{name}: expected exactly one item, found {counts[name]}")
    imports = "".join(block for _, kind, block, _ in original if kind == "use")
    groups = {module: [] for module in manifest}
    remaining = []
    for item in original:
        module = assignments.get(item[0])
        if module:
            groups[module].append(item)
        else:
            remaining.append(item)
    check_conservation(
        [item[2] for item in original], [item[2] for item in remaining],
        [item[2] for group in groups.values() for item in group],
    )
    directory = source_path.parent if source_path.stem in ("lib", "main", "mod") else source_path.with_suffix("")
    parent = {id(item): item[2] for item in remaining}
    additions = []
    outputs = {}
    for module, group in groups.items():
        destination = directory / f"{module}.rs"
        if destination.exists() or destination.is_symlink():
            raise ValueError(f"destination exists: {destination}")
        if len(group) == 1 and group[0][0:2] == (module, "mod"):
            # Inline module -> file module: retain its attributes and lexical nesting.
            item = group[0]
            block, offset = item[2:]
            opening = re.match(r"(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{", block[offset:])
            if opening is None:
                raise ValueError(f"{module}: expected inline module")
            begin = offset + opening.end()
            end = block.rfind("}")
            body = block[begin:end]
            declaration = block[:begin - 1].rstrip() + ";" + block[end + 1:]
            outputs[destination] = body
            parent[id(item)] = declaration
        else:
            if counts[module]:
                raise ValueError(f"module name already used in parent: {module}")
            moved = []
            for _, kind, block, offset in group:
                if kind != "impl" and not re.match(r"pub\b", block[offset:]):
                    block = block[:offset] + "pub(super) " + block[offset:]
                moved.append(block)
            outputs[destination] = "use super::*;\n" + imports + "".join(moved)
            additions.append(f"mod {module};\nuse {module}::*;\n")
    outputs[source_path] = prefix + "".join(parent.get(id(item), "") for item in original) + "".join(additions)
    # All parsing, manifest and conservation checks finish before the first write.
    for path, content in outputs.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    return list(outputs)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    try:
        for path in split(args.source, args.manifest):
            print(path)
    except (ValueError, OSError) as error:
        parser.error(str(error))
