#!/usr/bin/env python3
"""Compare production source tokens between a qualified revision and a freeze candidate."""
import argparse
from pathlib import Path
import re
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent / "codemods"))
from rs import close, mask

ATTR = re.compile(r'#\[\s*(?:test\s*\]|cfg\s*\(\s*(?:all\s*\(\s*)?test\b|doc\s*=)|#!\[\s*deny\s*\(\s*missing_docs\s*\)\s*\]')
DOC_OR_COMMENT = re.compile(r'//[^\n]*|/\*')
LEXEME = re.compile(r'''(?:b|c)?r(?P<hashes>\#*)"|(?:b|c)?"|(?:b)?'(?:\\(?:u\{[\da-fA-F]+\}|x[\da-fA-F]{2}|.)|[^\\'\n])'|\w+|[^\s]''')


def runtime_tokens(source):
    """Ignore noncompiled tests, docs and the docs lint. Retain literal contents."""
    code = mask(source)
    edits=[]
    for attr in ATTR.finditer(code):
        if edits and attr.start() < edits[-1][1]:
            continue
        end=close(code,code.index('[',attr.start()))+1
        if re.match(r'#\[\s*(?:cfg|test\b)',attr[0]):
            item=end
            while True:
                item+=len(code[item:])-len(code[item:].lstrip())
                if not code.startswith('#[',item):
                    break
                item=close(code,item+1)+1
            if not re.match(r'(?:pub(?:\(crate\))?\s+)?(?:mod\s+\w+\s*(?:[;{])|fn\s+\w+[^;{]*\{)',code[item:]):
                continue
            boundary=re.search(r'[;{]',code[item:]); assert boundary
            item+=boundary.start()
            end=close(code,item)+1 if code[item]=='{' else item+1
        edits.append((attr.start(),end))
    for start,end in reversed(edits): source=source[:start]+' '+source[end:]
    at=0
    while at<len(source):
        if source[at].isspace():
            at+=1; continue
        if comment:=DOC_OR_COMMENT.match(source,at):
            if comment[0]!='/*': at=comment.end(); continue
            at=comment.end(); depth=1
            while depth and at<len(source):
                if source.startswith('/*',at): depth+=1;at+=2
                elif source.startswith('*/',at): depth-=1;at+=2
                else: at+=1
            continue
        token=LEXEME.match(source,at); assert token
        end=token.end()
        if token.group('hashes') is not None:
            delim='"'+token['hashes']; found=source.find(delim,end)
            end=len(source) if found<0 else found+len(delim)
        elif token[0].endswith('"'):
            while end<len(source):
                if source[end]=='"': end+=1; break
                end+=2 if source[end]=='\\' else 1
        yield source[at:end]
        at=end


def invoke(repo,*args):
    return subprocess.check_output(['git','-C',str(repo),*args])


def ignored(path):
    return (path.suffix=='.md' or any(part in {'tools','docs','tests','specification'} for part in path.parts)
            or path.as_posix()=='pxui')


def differences(repo,before,after):
    paths=invoke(repo,'diff','--name-only','-z','--no-renames',before,*(() if after=='WORKTREE' else (after,))).decode().split('\0')
    for name in paths:
        if not name or ignored(Path(name)):
            continue
        def blob(rev,filename=name):
            if rev=='WORKTREE':
                path=repo/filename
                return path.read_bytes() if path.is_file() else b''
            return subprocess.run(['git','-C',str(repo),'show',f'{rev}:{filename}'],capture_output=True,check=False).stdout
        old,new=blob(before),blob(after)
        if name.endswith('.rs'):
            module=Path(name)
            module_name=module.parent.name if module.name=='mod.rs' else module.stem
            parent=module.parent.parent if module.name=='mod.rs' else module.parent
            declaration=re.compile(r'#\[cfg\((?:all\(\s*)?test[^\]\n]*\]\s*(?:#\[[^\n]*\]\s*)*mod\s+'+re.escape(module_name)+r'\s*;')
            if any(all(declaration.search(mask(blob(rev, (parent/base).as_posix()).decode())) for rev in (before,after))
                   for base in ('lib.rs','mod.rs','main.rs')):
                continue
            equal=list(runtime_tokens(old.decode()))==list(runtime_tokens(new.decode()))
        else:
            equal=old==new
        if not equal:
            yield name


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository',type=Path)
    parser.add_argument('baseline')
    parser.add_argument('candidate',nargs='?',default='HEAD')
    args=parser.parse_args()
    changed=list(differences(args.repository,args.baseline,args.candidate))
    for path in changed: print(f'Runtime changed: {path}')
    if not changed: print(f'Runtime matches: {args.repository} {args.baseline}..{args.candidate}')
    return bool(changed)


if __name__=='__main__':
    assert list(runtime_tokens('/// Doc\n#[doc = "Own"] pub fn x() {}\n#[cfg(test)] mod t { fn y() {} }'))==list(runtime_tokens('pub fn x() {}'))
    assert list(runtime_tokens('let x = "one two";'))!=list(runtime_tokens('let x = "onetwo";'))
    assert list(runtime_tokens('let x = r#"/// keep"#;'))!=list(runtime_tokens('let x = r#"/// lose"#;'))
    assert list(runtime_tokens('#[cfg(test)] mod t; pub fn x() {}'))==list(runtime_tokens('pub fn x() {}'))
    assert list(runtime_tokens('#[test] fn t() { assert!(true); } pub fn x() {}'))==list(runtime_tokens('pub fn x() {}'))
    assert list(runtime_tokens('#[cfg(not(test))] fn x() { called(); }'))!=list(runtime_tokens('#[cfg(not(test))] fn x() { removed(); }'))
    sys.exit(main())
