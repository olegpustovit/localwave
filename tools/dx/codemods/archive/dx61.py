import re
import sys

WT = "/Users/snowbear/WORK/GIT/dx-wt/dx6.1"
AG = f"{WT}/AGENTS.md"
ABI = f"{WT}/docs/abi.md"
SECTION = "tools/dx/codemods/archive/agents_section.md"

# (first line, last line, version) -- 1-based inclusive, from the AGENTS.md read.
RANGES = [
    (99, 102, "1.23"), (106, 112, "1.10"), (116, 122, "1.11"), (124, 131, "1.12"),
    (139, 147, "1.13"), (150, 154, "1.14"), (163, 169, "1.15"), (227, 231, "1.16"),
    (243, 248, "1.17"), (256, 261, "1.18"), (264, 267, "1.19"), (273, 280, "1.20"),
    (290, 294, "1.21"), (301, 304, "1.22"), (306, 310, "1.22"), (312, 314, "1.22"),
    (317, 323, "1.24"), (326, 331, "1.25"), (343, 351, "1.38"), (352, 356, "1.40"),
    (357, 360, "1.41"), (361, 362, "1.42"), (389, 395, "1.26"), (398, 409, "1.27"),
    (411, 416, "1.28"), (418, 424, "1.29"), (426, 431, "1.30"), (433, 435, "1.30"),
    (437, 444, "1.31"), (447, 452, "1.32"), (455, 462, "1.33"), (475, 479, "1.34"),
    (481, 485, "1.35"),
]

with open(AG, encoding="utf-8") as f:
    lines = f.read().split("\n")  # keep final "" if file ends with newline
assert lines[-1] == "", "AGENTS.md must end with newline"
lines = lines[:-1]

# Verify each range starts with an ABI version line or continues one, and ends where expected.
moved = {}
removed = set()
for start, end, ver in RANGES:
    block = lines[start - 1:end]
    assert block, (start, end)
    if start in (306, 312, 433):
        pass  # unprefixed continuations of ABI 1.22 / 1.30 text (checked by content below)
    else:
        assert block[0].startswith(f"ABI {ver} "), (start, block[0][:40])
    assert block[-1].strip() != "", (start, end)
    if start in (306, 312, 433):
        assert ("Ui::a11y_value" in block[0]) or ("accessibility-value ABI" in block[0]) or ("box_of_a11y" in block[0]), block[0][:60]
    moved.setdefault(ver, []).append("\n".join(block))
    for i in range(start, end + 1):
        assert i not in removed, i
        removed.add(i)

# Sanity: every ABI-prefixed line in AGENTS.md is covered by a moved range.
for i, l in enumerate(lines, 1):
    if re.match(r"ABI 1\.\d+ ", l):
        assert i in removed, ("uncovered ABI paragraph", i, l[:50])

# Build new AGENTS.md
new = [l for i, l in enumerate(lines, 1) if i not in removed]

# Remove the now-empty "## Numeric keyboard signals" heading (its body was only ABI 1.10 text).
idx = new.index("## Numeric keyboard signals")
j = idx + 1
while new[j] == "":
    j += 1
assert new[j].startswith("## "), new[j]
del new[idx:j]

text = "\n".join(new) + "\n"
# Collapse blank-line runs left behind by the removals.
text = re.sub(r"\n{3,}", "\n\n", text)

# Dead reference: specification/abi.md -> docs/abi.md
old_ref = "ABI design lives in `specification/abi.md`."
assert text.count(old_ref) == 1
text = text.replace(old_ref, "ABI design lives in `docs/abi.md`.")

# Contracts bullet sentence.
old_c = "- A warm frame allocates nothing; `mem_sys`'s counting allocator is what the tests assert against.\n"
assert text.count(old_c) == 1
new_c = old_c.rstrip("\n") + (
    " One named exception: shaping a string the text cache has never seen during a fast fling may\n"
    "  allocate inside the shaping library; it has its own measured workload whose ceiling may only go down.\n"
)
text = text.replace(old_c, new_c)

# Insert the public API section after Contracts, before Architecture.
section = open(SECTION, encoding="utf-8").read()
assert section.endswith("\n") and not section.endswith("\n\n")
anchor = "\n## Architecture: UI, platform and host integration\n"
assert text.count(anchor) == 1
text = text.replace(anchor, "\n" + section + anchor)

with open(AG, "w", encoding="utf-8") as f:
    f.write(text)

# docs/abi.md: append notes per version, numerically ordered, verbatim paragraphs.
def key(v):
    return tuple(int(x) for x in v.split("."))

abi = open(ABI, encoding="utf-8").read()
assert abi.endswith("\n")
out = [abi.rstrip("\n"), "", "## Version notes", ""]
for ver in sorted(moved, key=key):
    out.append(f"### {ver}")
    out.append("")
    for para in moved[ver]:
        out.append(para)
        out.append("")
text_abi = "\n".join(out).rstrip("\n") + "\n"
with open(ABI, "w", encoding="utf-8") as f:
    f.write(text_abi)

print("moved versions:", sorted(moved, key=key))
print("moved blocks:", sum(len(v) for v in moved.values()))
print("removed lines:", len(removed))
