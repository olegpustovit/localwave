//! Test frame driver shared by lw_ui's unit tests and lw_bin's test drivers; production never calls it.
//!
//! [`frame`] runs `Ui::frame` and, when `LW_FINGERPRINT=<path>` is set, appends the frame's fingerprint to
//! `<path>` so two runs can be compared with `tools/dx/gcmp.py`. Records are tab-separated, one block per
//! frame, written under a thread and file lock so parallel tests and processes never interleave:
//!
//! - `F test n instances geom full a11y`: the test (thread) name, the thread's frame number, the instance
//!   count and three FNV-1a 64 hashes (instance geometry/colour, every instance field, the a11y tree).
//! - `A test n role actions name`: one line per accessibility node with a name or an action (names with
//!   tabs or newlines are written with spaces).
//!
//! The dump allocates nothing (stack buffer, one file opened once per process), so allocation-counting
//! tests stay valid with fingerprints on.
use std::fs::File;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

use pxui::{FrameIn, FrameOut, Ui};

/// Runs one frame of `ui` (`Ui::frame`), appends its fingerprint when `LW_FINGERPRINT` is set and returns
/// the frame's output (damage; its layer pointer is valid until the next frame).
pub fn frame(
    ui: &mut Ui,
    input: &FrameIn,
    build: &mut dyn FnMut(&mut Ui),
) -> Result<FrameOut, pxui::Error> {
    let out = *ui.frame(input, build)?;
    static FILE: OnceLock<Option<Mutex<File>>> = OnceLock::new();
    let file = FILE.get_or_init(|| {
        std::env::var_os("LW_FINGERPRINT").map(|path| {
            Mutex::new(
                File::options()
                    .create(true)
                    .append(true)
                    .open(path)
                    .expect("LW_FINGERPRINT: cannot open"),
            )
        })
    });
    if let Some(file) = file {
        // The mutex orders this process's threads; the file lock orders processes (cargo t runs cases apart).
        let mut file = file.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        file.lock().expect("LW_FINGERPRINT: cannot lock");
        dump(ui, &mut file);
        let _ = file.unlock();
    }
    Ok(out)
}

thread_local! { static FRAME: core::cell::Cell<u64> = const { core::cell::Cell::new(0) }; }

fn dump(ui: &Ui, file: &mut File) {
    let n = FRAME.with(|c| c.replace(c.get() + 1));
    let thread = std::thread::current();
    let test = thread.name().unwrap_or("?");
    let (mut geom, mut full, mut a11y) = (Fnv::NEW, Fnv::NEW, Fnv::NEW);
    let mut count = 0usize;
    for layer in ui.layers() {
        for i in layer.instances {
            count += 1;
            for h in [&mut geom, &mut full] {
                for v in i.rect {
                    h.add(&v.to_bits().to_le_bytes());
                }
                for v in i.clip {
                    h.add(&v.to_le_bytes());
                }
                h.add(&i.color);
                h.add(&i.color2);
                for v in i.radii {
                    h.add(&v.to_le_bytes());
                }
                h.add(&i.width.to_le_bytes());
                h.add(&[i.kind, i.flags]);
            }
            for v in i.aux.into_iter().chain(i.extra) {
                full.add(&v.to_le_bytes());
            }
            full.add(&i.view.to_le_bytes());
            full.add(&i.clip_radius.to_le_bytes());
        }
    }
    for node in ui.a11y_nodes() {
        a11y.add(&node.role.to_le_bytes());
        a11y.add(ui.a11y_name_of(node).as_bytes());
        for v in [node.rect.x, node.rect.y, node.rect.w, node.rect.h] {
            a11y.add(&v.to_bits().to_le_bytes());
        }
        a11y.add(&node.states.to_le_bytes());
        a11y.add(&node.actions.to_le_bytes());
    }
    let mut out = Out {
        buf: [0; 4096],
        len: 0,
        file,
    };
    let _ = writeln!(
        out,
        "F\t{test}\t{n}\t{count}\t{:016x}\t{:016x}\t{:016x}",
        geom.0, full.0, a11y.0
    );
    for node in ui.a11y_nodes() {
        let name = ui.a11y_name_of(node);
        if name.is_empty() && node.actions == 0 {
            continue;
        }
        let _ = write!(out, "A\t{test}\t{n}\t{}\t{:x}\t", node.role, node.actions);
        for (k, part) in name.split(['\t', '\n']).enumerate() {
            let _ = out.write_all(if k == 0 { b"" } else { b" " });
            let _ = out.write_all(part.as_bytes());
        }
        let _ = out.write_all(b"\n");
    }
    let _ = out.flush();
}

struct Fnv(u64);

impl Fnv {
    const NEW: Fnv = Fnv(0xcbf2_9ce4_8422_2325);
    fn add(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ *b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
}

/// A stack-buffered writer: `File` writes in 4 KiB chunks without a heap buffer.
struct Out<'a> {
    buf: [u8; 4096],
    len: usize,
    file: &'a mut File,
}

impl Write for Out<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.len + bytes.len() > self.buf.len() {
            self.flush()?;
        }
        if bytes.len() > self.buf.len() {
            return self.file.write(bytes);
        }
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.write_all(&self.buf[..self.len])?;
        self.len = 0;
        Ok(())
    }
}
