#!/usr/bin/env python3
"""Apply the U1a Rust namespace changes to the supplied source files."""
import argparse
import re
from pathlib import Path

from rs import args, close, line_of, mask


SHIMS = {
    "ui_core": "ui",
    "ui_text": "ui",
    "ui_widgets": "ui",
}
PRIVATE = {
    "text": {"pragmasevka", "Text", "TextStyle", "UiText", "TextWorker", "PreparedText", "RetiredText", "TextAlign", "Ellipsis", "Wrap"},
    "paint": {"Paint", "Fill", "Stroke"},
    "widgets": {"Scrub"},
    "collections": {"TreeNode"},
    "image": {"ImageStyle"},
    "kit": {"Repeat", "Keyframe", "Keyframes", "Target"},
    "composite": set(),
    "overlays": set(),
}
A11Y = {"A11yAction", "A11yNode", "A11yTree", "Role"}
PATH = re.compile(r"\b(pxui|crate)::(\w+)::(\w+|\{[\w\s,]+\}|\*)")


def modules(source, path):
    in_wrapper = "crates/public/pxui/src/" in path.as_posix()
    # Include explicit paths in rustdoc examples as well as in code.
    visible = list(mask(source))
    for doc in re.finditer(r"^\s*//!.*$|^\s*///.*$", source, re.M):
        visible[doc.start():doc.end()] = doc[0]
    edits, skipped = [], []
    for match in PATH.finditer("".join(visible)):
        crate, module, item = match.groups()
        if crate == "crate" and not in_wrapper:
            continue
        if module == "a11y" and item in A11Y:
            replacement = f"{crate}::{item}"
        elif module == "a11y" and (item == "state" or item.strip("{}").replace(" ", "") == "state,Role"):
            replacement = f"{crate}::ui::{item}"
        elif module in SHIMS and item != "key":
            replacement = f"{crate}::{SHIMS[module]}::{item}"
        elif module in PRIVATE and item in PRIVATE[module]:
            destination = f"{crate}::ui" if item in {"Keyframe", "Target"} else crate
            replacement = f"{destination}::{item}"
        elif module == "a11y" or module in SHIMS or (crate == "pxui" and module in PRIVATE):
            skipped.append(f"{path}:{line_of(source, match.start())}: {match[0]}")
            continue
        else:
            continue
        edits.append((match.start(), match.end(), replacement))
    return edits, skipped


def themes(source, path):
    wrapper = "crate" if "crates/public/pxui/src/" in path.as_posix() else "pxui"
    visible = mask(source)
    edits, skipped = [], []
    # A source record with default flags and a single borrowed string.
    old_source = re.compile(r"\.set_theme(?:_source)?\(\s*&\s*(?:\w+::)*ThemeSource\s*\{")
    for match in old_source.finditer(visible):
        end = close(visible, visible.index("(", match.start()))
        record = visible[match.end():end]
        text = re.search(r"\bptr:\s*([\w:]+)\.as_ptr\(\)\s*,\s*len:\s*\1\.len\(\)", record)
        flags = re.search(r"\bflags:\s*([^,}]+)", record)
        if text and (not flags or flags[1].strip() == "0"):
            edits.append((match.start(), end + 1, f".set_theme(&{wrapper}::ThemeHandle::parse(&{text[1]}).unwrap())"))
        else:
            skipped.append(f"{path}:{line_of(source, match.start())}: theme source or flags")

    # Parse a scoped theme whose only mutation is applying a source.
    old_scope = re.compile(r"\{\s*let mut theme\s*=\s*(?:crate::|pxui::)?Theme::default\(\);\s*theme\.apply\(&ui,\s*")
    for match in old_scope.finditer(visible):
        end = close(visible, match.start())
        apply_end = close(visible, visible.rindex("(", match.start(), match.end()))
        suffix = source[apply_end + 1:end]
        if re.fullmatch(r"\.(?:expect\(.*?\)|unwrap\(\));\s*theme\s*", suffix, re.S):
            text = source[match.end():apply_end]
            edits.append((match.start(), end + 1, f"{wrapper}::ThemeHandle::parse({text}){suffix[:suffix.rindex(';')]}"))
        else:
            skipped.append(f"{path}:{line_of(source, match.start())}: scoped theme")
    return sorted(edits), skipped


def lifecycle(source, path, external=False):
    wrapper = "crate" if not external and "crates/public/pxui/src/" in path.as_posix() else "pxui"
    visible = mask(source)
    edits, skipped = [], []
    calls = re.compile(r"(?<!\w)(?:(pxui|crate)::)?Ui::(new|create)\(")
    for match in calls.finditer(visible):
        if visible[max(0, match.start() - 2):match.start()] == "::":
            continue
        start = match.end() - 1
        end = close(visible, start)
        call = [source[a:b].strip() for a, b in args(visible, start, end)]
        if match[2] == "new" and not call:
            edits.append((start + 1, end, f"&{wrapper}::UiDesc::DEFAULT"))
        elif match[2] == "create" and len(call) == 3:
            platform, size, fonts = call
            size = re.fullmatch(r"Some\((\d+)\)|None", size)
            if not size or not fonts.startswith("&[") or not fonts.endswith("]"):
                skipped.append(f"{path}:{line_of(source, match.start())}: Ui::create arguments")
                continue
            if platform.startswith("if "):
                skipped.append(f"{path}:{line_of(source, match.start())}: Ui::create platform")
                continue
            opened = source.index("[", args(visible, start, end)[2][0])
            font_args = [source[a:b].strip() for a, b in args(visible, opened, close(visible, opened))]
            fonts = ", ".join(f"{wrapper}::FontData({font})" for font in font_args)
            desc = f"{wrapper}::UiDesc::new({platform})"
            if size[1]:
                desc += f".font_size({size[1]}.0)"
            if fonts:
                desc += f".fonts(&[{fonts}])"
            ui = (match[1] + "::" if match[1] else "") + "Ui"
            edits.append((match.start(), end + 1, f'{ui}::new(&{desc}).expect("pxui Ui")'))

    literal = re.compile(r"(?<!\w)(?:(pxui_types|pxui|crate)::)?FrameIn\s*\{")
    for match in literal.finditer(visible):
        if visible[:match.start()].rstrip().endswith(("->", "::", "struct", "impl")):
            continue
        start = match.end() - 1
        end = close(visible, start)
        fields, base = {}, ""
        for a, b in args(visible, start, end):
            item = source[a:b].strip()
            if item.startswith(".."):
                base = item[2:].strip()
            elif field := re.match(r"(\w+)\s*:(?!:)\s*(.*)", item, re.S):
                fields[field[1]] = field[2]
            elif re.fullmatch(r"\w+", item):
                fields[item] = item
            else:
                skipped.append(f"{path}:{line_of(source, match.start())}: nonliteral frame fields")
                break
        else:
            scope = wrapper if match[1] == "pxui_types" else match[1]
            name = (scope + "::" if scope else "") + "FrameIn"
            if not base or re.fullmatch(r"(?:(?:\w+::)*FrameIn|Default)::default\(\)", base):
                w, h = fields.pop("width", "0.0"), fields.pop("height", "0.0")
                base = f"{name}::new({w}, {h})"
            if re.fullmatch(r"(?:\w+::)*size_of::<(?:\w+::)*FrameIn>\(\)\s+as u32", fields.get("size", "")):
                del fields["size"]
            for field in ("scale", "time", "environment", "keyboard"):
                if field in fields:
                    base += f".{field}({fields.pop(field)})"
            motion = fields.get("reduced_motion")
            if motion in ("0", "1"):
                del fields["reduced_motion"]
                base += f".reduced_motion({str(motion == '1').lower()})"
            if fields:
                variable = "frame"
                if re.search(r"\bframe\b", " ".join(fields.values())):
                    variable = "descriptor"
                line = source.rfind("\n", 0, match.start()) + 1
                indent = re.match(r"[ \t]*", source[line:])[0]
                statements = [f"let mut {variable} = {base};"]
                statements += [f"{variable}.wire.{key} = {value};" for key, value in fields.items()]
                statements.append(variable)
                base = "{\n" + "".join(f"{indent}    {s}\n" for s in statements) + indent + "}"
            edits.append((match.start(), end + 1, base))

    snapshot = re.compile(r"\.input_frame\(")
    for match in snapshot.finditer(visible):
        start = match.end() - 1
        arguments = args(visible, start, close(visible, start))
        if len(arguments) == 2:
            first, last = arguments[0]
            value = source[first:last].strip()
            edits.append((match.start(), last, f".frame(&unsafe {{ {wrapper}::FrameIn::snapshot({value}) }}"))
        else:
            skipped.append(f"{path}:{line_of(source, match.start())}: snapshot arguments")
    return sorted(edits), skipped


DOC_CODE = re.compile(r"^([ \t]*//[!/] )```(?:rust)?\n((?:(?![ \t]*//[!/] ```)[ \t]*//[!/] ?[^\n]*\n)+)\1```", re.M)


def lifecycle_docs(source, path):
    edits, skipped = lifecycle(source, path)
    for block in DOC_CODE.finditer(source):
        code = re.sub(r"^[ \t]*//[!/] ?", "", block[2], flags=re.M)
        migrated, omitted = lifecycle(code, path, external=True)
        skipped.extend(omitted)
        if migrated:
            for start, end, replacement in reversed(migrated):
                code = code[:start] + replacement + code[end:]
            edits.append((block.start(2), block.end(2), "".join(block[1] + line for line in code.splitlines(keepends=True))))
    return sorted(edits), skipped


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    phase = parser.add_mutually_exclusive_group()
    phase.add_argument("--themes", action="store_true")
    phase.add_argument("--lifecycle", action="store_true")
    parser.add_argument("files", nargs="+", type=Path)
    options = parser.parse_args()
    total = 0
    for path in options.files:
        source = path.read_text()
        rewrite = lifecycle_docs if options.lifecycle else themes if options.themes else modules
        edits, skipped = rewrite(source, path)
        for site in skipped:
            print("skipped", site)
        if edits:
            print(f"{path}: {len(edits)}")
            total += len(edits)
            if not options.dry_run:
                for start, end, replacement in reversed(edits):
                    source = source[:start] + replacement + source[end:]
                path.write_text(source)
    print(f"total {total}" + (" (dry run)" if options.dry_run else ""))


if __name__ == "__main__":
    main()
