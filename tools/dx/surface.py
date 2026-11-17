#!/usr/bin/env python3
"""The public surface of pxui and pxui_types, generated from rustdoc JSON (U0.2).

  surface.py            regenerate tools/dx/pxui_surface.txt
  surface.py --check    fail (exit 1) when tools/dx/pxui_surface.txt is stale
  surface.py --ledger [pxui/docs/api.md] [--all]
                        diff the surface against the ledger's machine list (part 2), both directions; lines
                        whose step has not landed are skipped (a step has landed when a commit subject on
                        HEAD of pxui or LocalWave starts with "[<step>"); exits 1 on any difference.
                        --all compares the whole target (the planned HEAD -> ledger delta) and exits 1 only
                        on untagged delta lines. Both write tools/dx/ledger_delta.txt.

One line per item: `<path> <kind> <signature>`, sorted. Items reachable from each crate root through
public modules and `pub use` re-exports, including macro-generated ones; inherent methods and associated
consts as `<Type>::<name>`, struct fields as `<Type>::<field>`, trait impls as `<Type> impl <Trait>`.
Functions render as `<path> fn [const ][unsafe ]<generics>(<params>) -> <ret>`. Types inside signatures use
their last path segment as written (`Rgba`, not `crate::geom::Rgba`).
Re-exports of another crate's item are one `use` line (that crate's own lines carry its surface).
Items hidden from docs are not part of the surface. Runs, inside pxui/:
  RUSTC_BOOTSTRAP=1 cargo rustdoc -p <crate> --lib -- -Z unstable-options --output-format json --cap-lints warn
"""
import json, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PXUI = os.path.join(ROOT, "pxui")
OUT = os.path.join(ROOT, "tools", "dx", "pxui_surface.txt")
DELTA = os.path.join(ROOT, "tools", "dx", "ledger_delta.txt")
CRATES = ("pxui", "pxui_types")
FORMAT_VERSION = 61  # rustdoc JSON of the pinned toolchain (pxui/rust-toolchain.toml); a change needs a review of the renderer


def rustdoc(crate):
    env = dict(os.environ, RUSTC_BOOTSTRAP="1")
    cmd = ["cargo", "rustdoc", "-q", "-p", crate, "--lib", "--", "-Z", "unstable-options", "--output-format", "json", "--cap-lints", "warn"]
    subprocess.run(cmd, cwd=PXUI, env=env, check=True, stderr=subprocess.DEVNULL)
    target = os.environ.get("CARGO_TARGET_DIR", os.path.join(PXUI, "target"))
    doc = json.load(open(os.path.join(target, "doc", crate + ".json")))
    assert doc["format_version"] == FORMAT_VERSION, f"rustdoc JSON format {doc['format_version']} != {FORMAT_VERSION}"
    return doc


def args(a):
    if not a:
        return ""
    if "angle_bracketed" in a:
        ab = a["angle_bracketed"]
        parts = []
        for x in ab["args"]:
            if "type" in x:
                parts.append(ty(x["type"]))
            elif "lifetime" in x:
                parts.append(x["lifetime"])
            elif "const" in x:
                parts.append(x["const"].get("expr", "_"))
            else:
                parts.append("_")
        for c in ab.get("constraints", []):
            b = c["binding"]
            parts.append(c["name"] + args(c.get("args")) + (" = " + ty(b["equality"]["type"]) if "equality" in b else ": " + bounds(b["constraint"])))
        return "<" + ", ".join(parts) + ">" if parts else ""
    if "parenthesized" in a:
        p = a["parenthesized"]
        return "(" + ", ".join(ty(t) for t in p["inputs"]) + ")" + (" -> " + ty(p["output"]) if p.get("output") else "")
    return ""


def path(p):
    # The last segment as source writes it (`crate::geom::Rgba` -> `Rgba`): the item column carries module paths.
    return p["path"].rsplit("::", 1)[-1] + args(p.get("args"))


def bounds(bs):
    out = []
    for b in bs:
        if "trait_bound" in b:
            t = b["trait_bound"]
            hr = "for<" + ", ".join(g["name"] for g in t["generic_params"]) + "> " if t.get("generic_params") else ""
            out.append(("?" if t.get("modifier") == "maybe" else "") + hr + path(t["trait"]))
        elif "outlives" in b:
            out.append(b["outlives"])
        else:
            out.append("_")
    return " + ".join(out)


def ty(t):
    if t is None:
        return "()"
    k, v = next(iter(t.items()))
    if k == "resolved_path":
        return path(v)
    if k in ("generic", "primitive"):
        return v
    if k == "borrowed_ref":
        return "&" + (v["lifetime"] + " " if v["lifetime"] else "") + ("mut " if v["is_mutable"] else "") + ty(v["type"])
    if k == "raw_pointer":
        return ("*mut " if v["is_mutable"] else "*const ") + ty(v["type"])
    if k == "slice":
        return "[" + ty(v) + "]"
    if k == "array":
        return "[" + ty(v["type"]) + "; " + v["len"] + "]"
    if k == "tuple":
        return "(" + ", ".join(ty(x) for x in v) + ("," if len(v) == 1 else "") + ")"
    if k == "dyn_trait":
        parts = [("for<" + ", ".join(g["name"] for g in p["generic_params"]) + "> " if p.get("generic_params") else "") + path(p["trait"]) for p in v["traits"]]
        return "dyn " + " + ".join(parts + ([v["lifetime"]] if v.get("lifetime") else []))
    if k == "impl_trait":
        return "impl " + bounds(v)
    if k == "function_pointer":
        s = v["sig"]
        hr = "for<" + ", ".join(g["name"] for g in v["generic_params"]) + "> " if v.get("generic_params") else ""
        h = v["header"]
        pre = ("unsafe " if h.get("is_unsafe") else "") + ("" if h.get("abi") == "Rust" else 'extern "%s" ' % abi(h.get("abi")))
        return hr + pre + "fn(" + ", ".join(ty(x[1]) for x in s["inputs"]) + ")" + (" -> " + ty(s["output"]) if s.get("output") else "")
    if k == "qualified_path":
        return "<" + ty(v["self_type"]) + (" as " + path(v["trait"]) if v.get("trait") else "") + ">::" + v["name"]
    if k == "infer":
        return "_"
    return k


def abi(a):
    return a if isinstance(a, str) else next(iter(a))


def generics(g):
    ps = []
    for p in g["params"]:
        k = p["kind"]
        if "lifetime" in k:
            ps.append(p["name"] + (": " + " + ".join(k["lifetime"]["outlives"]) if k["lifetime"]["outlives"] else ""))
        elif "type" in k:
            if k["type"].get("is_synthetic"):
                continue  # `impl Trait` argument, rendered in place
            b = bounds(k["type"]["bounds"])
            ps.append(p["name"] + (": " + b if b else ""))
        elif "const" in k:
            ps.append("const " + p["name"] + ": " + ty(k["const"]["type"]))
    where = []
    for w in g["where_predicates"]:
        if "bound_predicate" in w:
            where.append(ty(w["bound_predicate"]["type"]) + ": " + bounds(w["bound_predicate"]["bounds"]))
        elif "lifetime_predicate" in w:
            where.append(w["lifetime_predicate"]["lifetime"] + ": " + " + ".join(w["lifetime_predicate"]["outlives"]))
        elif "eq_predicate" in w:
            where.append(ty(w["eq_predicate"]["lhs"]) + " = " + ty(w["eq_predicate"]["rhs"].get("type")))
    return ("<" + ", ".join(ps) + ">" if ps else ""), (" where " + ", ".join(where) if where else "")


def fn(f):
    g, where = generics(f["generics"])
    h = f["header"]
    pre = ("const " if h["is_const"] else "") + ("unsafe " if h["is_unsafe"] else "") + ("" if abi(h["abi"]) == "Rust" else 'extern "%s" ' % abi(h["abi"]))
    ins = []
    for name, t in f["sig"]["inputs"]:
        if name == "self":
            r = ty(t)
            ins.append({"Self": "self", "&Self": "&self", "&mut Self": "&mut self"}.get(r, "self: " + r))
        else:
            ins.append(name + ": " + ty(t))
    out = f["sig"]["output"]
    return pre + g + "(" + ", ".join(ins) + ")" + (" -> " + ty(out) if out else "") + where


class Surface:
    def __init__(self, doc):
        self.doc, self.idx, self.lines, self.seen = doc, doc["index"], set(), set()

    def walk(self, module, prefix):
        if (module["id"], prefix) in self.seen:
            return
        self.seen.add((module["id"], prefix))
        for i in module["inner"]["module"]["items"]:
            item = self.idx[str(i)]
            if item["visibility"] != "public":
                continue
            inner = item["inner"]
            if "use" in inner:
                u = inner["use"]
                target = self.idx.get(str(u["id"])) if u["id"] is not None else None
                if u["is_glob"]:
                    if target and "module" in target["inner"]:
                        self.walk(target, prefix)
                    else:
                        self.lines.add(f"{prefix}::* use {u['source']}::*")
                elif target:
                    self.item(target, f"{prefix}::{u['name']}")
                else:
                    self.lines.add(f"{prefix}::{u['name']} use {u['source']}")
            else:
                self.item(item, f"{prefix}::{item['name']}")

    def item(self, item, p):
        k, v = next(iter(item["inner"].items()))
        add = self.lines.add
        if k == "module":
            add(f"{p} mod")
            self.walk(item, p)
        elif k == "function":
            add(f"{p} fn {fn(v)}")
        elif k in ("struct", "union", "enum"):
            g, where = generics(v["generics"])
            if k == "struct" and "tuple" in v["kind"]:
                fields = ", ".join(ty(self.idx[str(f)]["inner"]["struct_field"]) if f is not None else "_" for f in v["kind"]["tuple"])
                add(f"{p} struct {g}({fields}){where}")
            else:
                add(f"{p} {k} {g}{where}".rstrip())
            fields = v["kind"].get("plain", {}).get("fields", []) if k == "struct" else v.get("fields", []) if k == "union" else []
            for f in fields:
                fi = self.idx[str(f)]
                if fi["visibility"] == "public":
                    add(f"{p}::{fi['name']} field {ty(fi['inner']['struct_field'])}")
            for var in v.get("variants", []):
                vi = self.idx[str(var)]
                vk = vi["inner"]["variant"]["kind"]
                shape = ""
                if isinstance(vk, dict) and "tuple" in vk:
                    shape = "(" + ", ".join(ty(self.idx[str(f)]["inner"]["struct_field"]) if f is not None else "_" for f in vk["tuple"]) + ")"
                elif isinstance(vk, dict) and "struct" in vk:
                    shape = " { " + ", ".join(self.idx[str(f)]["name"] + ": " + ty(self.idx[str(f)]["inner"]["struct_field"]) for f in vk["struct"]["fields"]) + " }"
                disc = vi["inner"]["variant"].get("discriminant")
                add(f"{p}::{vi['name']} variant {shape.strip()}".rstrip() + (f" = {disc['expr']}" if disc else ""))
            self.impls(v.get("impls", []), p)
        elif k == "trait":
            g, where = generics(v["generics"])
            add(f"{p} trait {g}" + (": " + bounds(v["bounds"]) if v["bounds"] else "") + where)
            for m in v["items"]:
                self.assoc(self.idx[str(m)], p)
        elif k == "constant":
            add(f"{p} const {ty(v['type'])}")
        elif k == "static":
            add(f"{p} static {'mut ' if v['is_mutable'] else ''}{ty(v['type'])}")
        elif k == "type_alias":
            g, where = generics(v["generics"])
            add(f"{p} type {g}= {ty(v['type'])}{where}")
        elif k == "macro":
            add(f"{p} macro")
        else:
            add(f"{p} {k}")

    def impls(self, ids, p):
        for i in ids:
            im = self.idx[str(i)]["inner"]["impl"]
            if im["is_synthetic"] or im["blanket_impl"] is not None:
                continue
            if im["trait"] is None:
                for m in im["items"]:
                    mi = self.idx[str(m)]
                    if mi["visibility"] == "public":
                        self.assoc(mi, p)
            else:
                self.lines.add(f"{p} impl {'!' if im['is_negative'] else ''}{path(im['trait'])}")

    def assoc(self, mi, p):
        k, v = next(iter(mi["inner"].items()))
        if k == "function":
            self.lines.add(f"{p}::{mi['name']} fn {fn(v)}")
        elif k == "assoc_const":
            self.lines.add(f"{p}::{mi['name']} const {ty(v['type'])}")
        elif k == "assoc_type":
            self.lines.add(f"{p}::{mi['name']} type" + (" = " + ty(v["type"]) if v.get("type") else ""))


def generate():
    lines = set()
    for crate in CRATES:
        doc = rustdoc(crate)
        s = Surface(doc)
        root = doc["index"][str(doc["root"])]
        s.lines.add(f"{crate} mod")
        s.walk(root, crate)
        lines |= s.lines
    return "".join(l + "\n" for l in sorted(lines))


TAG = re.compile(r"^(-?)(.*?) @(HEAD|U[0-9][0-9a-z.]*)$")


def landed(step):
    """True when a commit subject on HEAD (LocalWave or pxui) starts with `[<step>` (e.g. [U1a.2] or [U1a.2/3])."""
    for repo in (ROOT, PXUI):
        log = subprocess.run(["git", "log", "--format=%s", "--fixed-strings", "--grep", "[" + step], cwd=repo, capture_output=True, text=True).stdout
        if any(re.match(r"\[%s[\]/ ]" % re.escape(step), s) for s in log.splitlines()):
            return True
    return False


def ledger(api, surface, everything):
    """Delta lines between the surface and the ledger's machine list, each with its step tag when it has one.

    Target T = @HEAD lines, landed @U lines, and unlanded deletions (still expected). A surface line missing
    from T is `- line`, tagged with the step of a ledger line of the same path (a pending or landed change);
    with landed steps only, lines whose path has a pending step are skipped. A T line missing from the
    surface is `+ line @step`. Untagged delta lines are what the ledger must still explain."""
    text = open(api).read()
    body = text[text.index("<!-- surface -->"):].split("```")[1].split("\n", 1)[1]
    target, pending, step_of, untagged, cache = set(), set(), {}, [], {}
    for raw in body.splitlines():
        if not raw.strip():
            continue
        m = TAG.match(raw)
        if not m:
            untagged.append(f"? untagged: {raw}")
            continue
        minus, line, step = m.groups()
        p = line.split(" ", 1)[0]
        done = step == "HEAD" or everything or cache.setdefault(step, landed(step))
        if step != "HEAD":
            step_of.setdefault(p, step)
            if not done:
                pending.add(p)
        if (minus and not done) or (not minus and done):
            target.add((line, step))
    lines = {l for l, _ in target}
    have = set(surface.splitlines())
    out = []
    for l in sorted(have - lines):
        p = l.split(" ", 1)[0]
        if p in pending and not everything:
            continue
        out.append(f"- {l} @{step_of[p]}" if p in step_of else f"- {l} (untagged)")
    for l, step in sorted(target):
        if l not in have:
            out.append(f"+ {l} @{step}" if step != "HEAD" else f"+ {l} (untagged: HEAD line missing)")
    return out + untagged


if __name__ == "__main__":
    argv = sys.argv[1:]
    surface = generate()
    if "--check" in argv:
        current = open(OUT).read() if os.path.exists(OUT) else ""
        if current != surface:
            sys.stderr.write("tools/dx/pxui_surface.txt is stale; run tools/dx/surface.py\n")
            sys.exit(1)
        print("surface ok:", surface.count("\n"), "items")
    elif "--ledger" in argv:
        rest = [a for a in argv if a not in ("--ledger", "--all")]
        everything = "--all" in argv
        out = ledger(rest[0] if rest else os.path.join(PXUI, "docs", "api.md"), surface, everything)
        open(DELTA, "w").write("".join(l + "\n" for l in out))
        bad = [l for l in out if "untagged" in l]
        print(f"ledger delta: {len(out)} lines ({sum(l.startswith('-') for l in out)} -, {sum(l.startswith('+') for l in out)} +), {len(bad)} untagged; written to tools/dx/ledger_delta.txt")
        # --all lists the planned delta: only untagged lines fail. Landed-only: any difference fails.
        sys.exit(1 if (bad if everything else out) else 0)
    else:
        open(OUT, "w").write(surface)
        print("wrote", OUT, surface.count("\n"), "items")
