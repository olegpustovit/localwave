//! Swift-compatible waveform rows; decoding and source access belong to a worker.
use lw_apple_sys::sqlite::{Connection, Error, Result, Value};
use std::fmt::Write;

pub const PEAK_COUNT: usize = 512;
pub type Peaks = [f32; PEAK_COUNT];
pub struct Cached {
    pub signature: String,
    pub peaks: Peaks,
}
pub struct Repository {
    db: Connection,
}
impl Repository {
    pub fn open(path: &str) -> Result<Self> {
        Self::new(Connection::open(path)?)
    }
    pub fn new(db: Connection) -> Result<Self> {
        db.execute_batch("CREATE TABLE IF NOT EXISTS waveforms(song_key INTEGER PRIMARY KEY,source_signature TEXT NOT NULL,peaks BLOB NOT NULL)")?;
        Ok(Self { db })
    }
    pub fn get(&self, song_key: i64) -> Result<Option<Cached>> {
        let mut found = None;
        let mut corrupt = false;
        self.db.query("SELECT source_signature,CASE WHEN length(peaks)<=65536 THEN peaks ELSE NULL END FROM waveforms WHERE song_key=?", &[Value::Integer(song_key)], &mut |row| {
            let signature = row.text(0).ok().flatten();
            let peaks = row.blob(1).ok().flatten().and_then(decode);
            if let (Some(signature), Some(peaks)) = (signature, peaks) { found = Some(Cached { signature: signature.to_owned(), peaks }); }
            else { corrupt = true; }
            Ok(())
        })?;
        if corrupt {
            self.db.execute(
                "DELETE FROM waveforms WHERE song_key=?",
                &[Value::Integer(song_key)],
            )?;
        }
        Ok(found)
    }
    pub fn put(&self, song_key: i64, signature: &str, peaks: &Peaks) -> Result<()> {
        let bytes = encode(peaks)?;
        self.db.execute("INSERT INTO waveforms(song_key,source_signature,peaks) VALUES(?,?,?) ON CONFLICT(song_key) DO UPDATE SET source_signature=excluded.source_signature,peaks=excluded.peaks", &[Value::Integer(song_key), Value::Text(signature), Value::Blob(bytes.as_bytes())])?;
        Ok(())
    }
}
fn encode(peaks: &Peaks) -> Result<String> {
    if peaks
        .iter()
        .any(|peak| !peak.is_finite() || !(0.0..=1.0).contains(peak))
    {
        return Err(Error::invalid("Invalid waveform peaks"));
    }
    let mut json = String::with_capacity(PEAK_COUNT * 12);
    json.push('[');
    for (index, peak) in peaks.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        write!(json, "{peak}").expect("Writing to a String cannot fail");
    }
    json.push(']');
    Ok(json)
}
fn whitespace(bytes: &[u8], position: &mut usize) {
    while bytes
        .get(*position)
        .is_some_and(|byte| matches!(byte, b' ' | b'\r' | b'\n' | b'\t'))
    {
        *position += 1;
    }
}
/// Decode only the fixed JSON number array, without a general parser or heap allocation.
fn decode(bytes: &[u8]) -> Option<Peaks> {
    if bytes.len() > 64 * 1024 {
        return None;
    }
    let mut position = 0;
    whitespace(bytes, &mut position);
    if bytes.get(position) != Some(&b'[') {
        return None;
    }
    position += 1;
    let mut peaks: Peaks = [0.0; PEAK_COUNT];
    for (index, peak) in peaks.iter_mut().enumerate() {
        whitespace(bytes, &mut position);
        if index != 0 {
            if bytes.get(position) != Some(&b',') {
                return None;
            }
            position += 1;
            whitespace(bytes, &mut position);
        }
        let start = position;
        if bytes.get(position) == Some(&b'-') {
            position += 1;
        }
        match bytes.get(position)? {
            b'0' => position += 1,
            b'1'..=b'9' => {
                position += 1;
                while bytes.get(position).is_some_and(u8::is_ascii_digit) {
                    position += 1;
                }
            }
            _ => return None,
        }
        if bytes.get(position) == Some(&b'.') {
            position += 1;
            let fraction = position;
            while bytes.get(position).is_some_and(u8::is_ascii_digit) {
                position += 1;
            }
            if fraction == position {
                return None;
            }
        }
        if matches!(bytes.get(position), Some(b'e' | b'E')) {
            position += 1;
            if matches!(bytes.get(position), Some(b'+' | b'-')) {
                position += 1;
            }
            let exponent = position;
            while bytes.get(position).is_some_and(u8::is_ascii_digit) {
                position += 1;
            }
            if exponent == position {
                return None;
            }
        }
        *peak = std::str::from_utf8(&bytes[start..position])
            .ok()?
            .parse()
            .ok()?;
        if !peak.is_finite() || !(0.0..=1.0).contains(peak) {
            return None;
        }
    }
    whitespace(bytes, &mut position);
    if bytes.get(position) != Some(&b']') {
        return None;
    }
    position += 1;
    whitespace(bytes, &mut position);
    (position == bytes.len()).then_some(peaks)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_json_peaks_reject_corruption_and_decode_without_allocations() {
        let peaks = std::array::from_fn(|i| i as f32 / (PEAK_COUNT - 1) as f32);
        let bytes = encode(&peaks).unwrap();
        assert_eq!(decode(bytes.as_bytes()), Some(peaks));
        let scientific = format!("[{}]", [" \t-0.0e+00\r\n"; PEAK_COUNT].join(","));
        assert_eq!(decode(scientific.as_bytes()), Some([0.0; PEAK_COUNT]));
        for bad in [
            "+0", "01", "0.", ".1", "NaN", "1e", "1e+", "1e999", "2", "-1", "null", "true",
            "\"0\"", "0\u{a0}",
        ] {
            let invalid = format!("[{}]", [bad; PEAK_COUNT].join(","));
            assert!(decode(invalid.as_bytes()).is_none(), "{bad}");
        }
        assert!(decode(b"[]").is_none());
        assert!(decode(format!("{bytes} trailing").as_bytes()).is_none());
        assert!(decode(&vec![b' '; 65537]).is_none());
        mem_sys::local_reset();
        for _ in 0..100 {
            std::hint::black_box(decode(bytes.as_bytes())).unwrap();
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
    #[test]
    fn waveform_rows_reopen_replace_and_recover_corruption_without_touching_audio() {
        let path = std::env::temp_dir().join(format!(
            "lw-waveform-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _cleanup = Cleanup(path.clone());
        let repo = Repository::open(path.to_str().unwrap()).unwrap();
        let mut peaks = [0.0; PEAK_COUNT];
        peaks[256..].fill(1.0);
        repo.put(42, "first", &peaks).unwrap();
        drop(repo);
        let repo = Repository::open(path.to_str().unwrap()).unwrap();
        assert_eq!(repo.get(42).unwrap().unwrap().peaks, peaks);
        repo.put(42, "second", &[0.0; PEAK_COUNT]).unwrap();
        assert_eq!(repo.get(42).unwrap().unwrap().signature, "second");
        for value in [
            Value::Blob(b"invalid JSON"),
            Value::Blob(b"[]"),
            Value::Integer(1),
            Value::Blob(&[]),
            Value::Blob(&[b'0'; 65537]),
        ] {
            repo.db
                .execute("UPDATE waveforms SET peaks=? WHERE song_key=42", &[value])
                .unwrap();
            assert!(repo.get(42).unwrap().is_none());
            repo.put(42, "recovered", &peaks).unwrap();
        }
        let mut invalid = peaks;
        invalid[0] = f32::NAN;
        assert!(repo.put(42, "bad", &invalid).is_err());
        assert_eq!(repo.get(42).unwrap().unwrap().signature, "recovered");
    }
}
