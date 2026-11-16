//! SBAsset6 (`.pak`) reading and writing.
//!
//! Layout: `"SBAsset6"`, u64 BE offset of the index, file data…, then at the index offset:
//! `"INDEX"`, VLQ count + (string key, binary-json value) metadata pairs, VLQ count +
//! (string path, u64 BE offset, u64 BE length) entries. Files are stored uncompressed.
//! The reader is adapted from SB_Songmaker's `starbound.rs`, but parses the index once.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Pak {
    pub path: PathBuf,
    file: Mutex<File>,
    index: HashMap<String, (u64, u64)>,
    pub metadata: serde_json::Map<String, Value>,
}

impl Pak {
    pub fn open(path: &Path) -> Result<Pak> {
        let mut f = File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut head = [0u8; 16];
        f.read_exact(&mut head)?;
        if &head[..8] != b"SBAsset6" {
            bail!("{} is not an SBAsset6 pak", path.display());
        }
        let idx_off = u64::from_be_bytes(head[8..16].try_into().unwrap());
        f.seek(SeekFrom::Start(idx_off))?;
        let mut idx = Vec::new();
        f.read_to_end(&mut idx)?;
        let mut c = Cur { b: &idx, p: 0 };
        if c.bytes(5)? != b"INDEX" {
            bail!("pak index missing");
        }
        let mut metadata = serde_json::Map::new();
        for _ in 0..c.vlq()? {
            let k = c.string()?;
            let v = c.json()?;
            metadata.insert(k, v);
        }
        let n = c.vlq()? as usize;
        let mut index = HashMap::with_capacity(n);
        for _ in 0..n {
            let p = c.string()?;
            let off = c.u64()?;
            let len = c.u64()?;
            index.insert(p, (off, len));
        }
        Ok(Pak { path: path.to_path_buf(), file: Mutex::new(f), index, metadata })
    }

    pub fn contains(&self, path: &str) -> bool {
        self.index.contains_key(path)
    }

    pub fn paths(&self) -> impl Iterator<Item = &String> {
        self.index.keys()
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>> {
        let &(off, len) = self.index.get(path).ok_or_else(|| anyhow!("{path} not in pak"))?;
        let mut f = self.file.lock().unwrap();
        f.seek(SeekFrom::Start(off))?;
        let mut out = vec![0u8; len as usize];
        f.read_exact(&mut out)?;
        Ok(out)
    }
}

struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl Cur<'_> {
    fn byte(&mut self) -> Result<u8> {
        let v = *self.b.get(self.p).ok_or_else(|| anyhow!("pak index truncated"))?;
        self.p += 1;
        Ok(v)
    }
    fn vlq(&mut self) -> Result<u64> {
        let mut v = 0u64;
        loop {
            let x = self.byte()?;
            v = (v << 7) | (x & 0x7f) as u64;
            if x & 0x80 == 0 {
                return Ok(v);
            }
        }
    }
    fn svlq(&mut self) -> Result<i64> {
        let v = self.vlq()?;
        Ok(if v & 1 != 0 { -((v >> 1) as i64) - 1 } else { (v >> 1) as i64 })
    }
    fn bytes(&mut self, n: usize) -> Result<&[u8]> {
        let s = self.b.get(self.p..self.p + n).ok_or_else(|| anyhow!("pak index truncated"))?;
        self.p += n;
        Ok(s)
    }
    fn string(&mut self) -> Result<String> {
        let n = self.vlq()? as usize;
        Ok(String::from_utf8_lossy(self.bytes(n)?).into_owned())
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    /// One value in Starbound's binary JSON encoding.
    fn json(&mut self) -> Result<Value> {
        Ok(match self.byte()? {
            1 => Value::Null,
            2 => {
                let f = f64::from_be_bytes(self.bytes(8)?.try_into().unwrap());
                serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null)
            }
            3 => Value::Bool(self.byte()? != 0),
            4 => Value::from(self.svlq()?),
            5 => Value::String(self.string()?),
            6 => {
                let n = self.vlq()?;
                let mut a = Vec::new();
                for _ in 0..n {
                    a.push(self.json()?);
                }
                Value::Array(a)
            }
            7 => {
                let n = self.vlq()?;
                let mut m = serde_json::Map::new();
                for _ in 0..n {
                    let k = self.string()?;
                    m.insert(k, self.json()?);
                }
                Value::Object(m)
            }
            t => bail!("unknown json type {t} in pak metadata"),
        })
    }
}

fn put_vlq(out: &mut Vec<u8>, mut v: u64) {
    let mut tmp = [0u8; 10];
    let mut i = tmp.len();
    loop {
        i -= 1;
        tmp[i] = (v & 0x7f) as u8 | if i == tmp.len() - 1 { 0 } else { 0x80 };
        v >>= 7;
        if v == 0 {
            break;
        }
    }
    out.extend_from_slice(&tmp[i..]);
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    put_vlq(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

fn put_json(out: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => out.push(1),
        Value::Bool(b) => {
            out.push(3);
            out.push(*b as u8);
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                out.push(4);
                let z = if i < 0 { (((-(i + 1)) as u64) << 1) | 1 } else { (i as u64) << 1 };
                put_vlq(out, z);
            } else {
                out.push(2);
                out.extend_from_slice(&n.as_f64().unwrap_or(0.0).to_be_bytes());
            }
        }
        Value::String(s) => {
            out.push(5);
            put_str(out, s);
        }
        Value::Array(a) => {
            out.push(6);
            put_vlq(out, a.len() as u64);
            a.iter().for_each(|x| put_json(out, x));
        }
        Value::Object(m) => {
            out.push(7);
            put_vlq(out, m.len() as u64);
            for (k, x) in m {
                put_str(out, k);
                put_json(out, x);
            }
        }
    }
}

/// Writes an SBAsset6 pak. `files` are (asset path starting with '/', bytes).
pub fn write_pak(out_path: &Path, files: &[(String, Vec<u8>)], metadata: &serde_json::Map<String, Value>) -> Result<()> {
    let mut data = Vec::new();
    data.extend_from_slice(b"SBAsset6");
    data.extend_from_slice(&[0u8; 8]);
    let mut entries = Vec::with_capacity(files.len());
    for (p, bytes) in files {
        entries.push((p.clone(), data.len() as u64, bytes.len() as u64));
        data.extend_from_slice(bytes);
    }
    let idx_off = data.len() as u64;
    data[8..16].copy_from_slice(&idx_off.to_be_bytes());
    data.extend_from_slice(b"INDEX");
    put_vlq(&mut data, metadata.len() as u64);
    for (k, v) in metadata {
        put_str(&mut data, k);
        put_json(&mut data, v);
    }
    put_vlq(&mut data, entries.len() as u64);
    for (p, off, len) in entries {
        put_str(&mut data, &p);
        data.extend_from_slice(&off.to_be_bytes());
        data.extend_from_slice(&len.to_be_bytes());
    }
    let tmp = out_path.with_extension("pak.tmp");
    File::create(&tmp)?.write_all(&data)?;
    std::fs::rename(&tmp, out_path)?;
    Ok(())
}

/// Packs a mod folder the way `asset_packer` does: every file becomes an asset,
/// `_metadata`/`.metadata` becomes the pak metadata.
pub fn pack_dir(dir: &Path, out_path: &Path, skip: &[&str]) -> Result<usize> {
    let mut files = Vec::new();
    let mut metadata = serde_json::Map::new();
    for e in walkdir::WalkDir::new(dir).sort_by_file_name() {
        let e = e?;
        if !e.file_type().is_file() {
            continue;
        }
        let rel = e.path().strip_prefix(dir)?.to_string_lossy().replace('\\', "/");
        if skip.iter().any(|s| rel == *s || rel.starts_with(&format!("{s}/"))) {
            continue;
        }
        let bytes = std::fs::read(e.path())?;
        if rel == "_metadata" || rel == ".metadata" {
            if let Value::Object(m) = crate::sbjson::parse(&bytes)? {
                metadata = m;
            }
            continue;
        }
        files.push((format!("/{rel}"), bytes));
    }
    let n = files.len();
    write_pak(out_path, &files, &metadata)?;
    Ok(n)
}

/// A folder name for unpacking a pak: its metadata `name`, else `friendlyName`, else the
/// file name (or, for a Steam workshop `contents.pak`, the workshop id folder).
pub fn suggested_dir_name(pak: &Pak) -> String {
    let from_meta = ["name", "friendlyName"].iter().find_map(|k| pak.metadata.get(*k).and_then(Value::as_str).map(String::from));
    let stem = pak.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let raw = from_meta.unwrap_or_else(|| {
        if stem == "contents" {
            pak.path.parent().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or(stem)
        } else {
            stem
        }
    });
    let clean: String = raw.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let clean = clean.trim_matches('_').to_string();
    if clean.is_empty() { "unpacked_mod".into() } else { clean }
}

/// Unpacks every asset into `dest` (which must not exist yet or be empty). When the pak has no
/// `_metadata` file of its own, one is written from the pak's index metadata, so the folder is
/// a regular mod again. Returns the number of files written.
pub fn unpack(pak: &Pak, dest: &Path) -> Result<usize> {
    if dest.exists() && std::fs::read_dir(dest)?.next().is_some() {
        bail!("{} already exists and is not empty", dest.display());
    }
    std::fs::create_dir_all(dest)?;
    let mut n = 0;
    let mut paths: Vec<&String> = pak.paths().collect();
    paths.sort();
    for p in paths {
        let target = crate::project::safe_join(dest, p)?;
        if let Some(d) = target.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&target, pak.read(p)?)?;
        n += 1;
    }
    if !dest.join("_metadata").exists() && !dest.join(".metadata").exists() {
        std::fs::write(dest.join("_metadata"), crate::sbjson::pretty(&Value::Object(pak.metadata.clone())))?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("sme-pak-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.pak");
        let mut meta = serde_json::Map::new();
        meta.insert("name".into(), "x".into());
        meta.insert("priority".into(), Value::from(-5));
        meta.insert("v".into(), serde_json::json!([1.5, true, null, {"a": "b"}]));
        let files = vec![("/a.txt".to_string(), b"hello".to_vec()), ("/d/b.bin".to_string(), vec![0u8; 300])];
        write_pak(&p, &files, &meta).unwrap();
        let pak = Pak::open(&p).unwrap();
        assert_eq!(pak.read("/a.txt").unwrap(), b"hello");
        assert_eq!(pak.read("/d/b.bin").unwrap().len(), 300);
        assert_eq!(Value::Object(pak.metadata.clone()), Value::Object(meta));
        std::fs::remove_dir_all(&dir).ok();
    }
}
