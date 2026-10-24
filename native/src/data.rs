// SPDX-License-Identifier: GPL-3.0-or-later
//! NumPy storage: directory-only NPZ indexing, on-demand extraction and mapped reads.
use anyhow::{Context, Result, bail, ensure};
use memmap2::{Mmap, MmapOptions};
use std::{
    fs::File,
    io::{BufReader, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub zip_index: Option<usize>,
    pub bytes: u64,
    pub compressed_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct Catalog {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}

impl Catalog {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path).context("Cannot open file")?;
        let mut magic = [0; 6];
        file.read_exact(&mut magic).context("File is too short")?;
        let entries = if &magic == b"\x93NUMPY" {
            vec![Entry {
                name: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                zip_index: None,
                bytes: file.metadata()?.len(),
                compressed_bytes: file.metadata()?.len(),
            }]
        } else {
            let mut zip = zip::ZipArchive::new(file).context("Not a valid NPY / NPZ file")?;
            let mut entries = Vec::new();
            for i in 0..zip.len() {
                let member = zip.by_index_raw(i)?;
                let member_name = member.name().context("Invalid NPZ entry name")?;
                if !member.is_dir() && member_name.ends_with(".npy") {
                    entries.push(Entry {
                        name: member_name.strip_suffix(".npy").unwrap().to_owned(),
                        zip_index: Some(i),
                        bytes: member.size(),
                        compressed_bytes: member.compressed_size(),
                    });
                }
            }
            ensure!(!entries.is_empty(), "Archive contains no .npy arrays");
            entries
        };
        Ok(Self {
            path: path.to_owned(),
            entries,
        })
    }
}

#[derive(Clone, Debug)]
enum Literal {
    Str(String),
    Int(usize),
    Bool(bool),
    Seq(Vec<Literal>),
    Dict(Vec<(String, Literal)>),
    None,
}

/// Small bounded Python-literal parser. Never evaluates Python or unpickles objects.
struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }
    fn value(&mut self, depth: usize) -> Result<Literal> {
        ensure!(depth < 64, "NPY header nesting is too deep");
        self.ws();
        match self
            .bytes
            .get(self.pos)
            .copied()
            .context("Incomplete NPY header")?
        {
            b'\'' | b'"' => Ok(Literal::Str(self.string()?)),
            b'(' | b'[' => {
                let close = if self.bytes[self.pos] == b'(' {
                    b')'
                } else {
                    b']'
                };
                self.pos += 1;
                let mut values = Vec::new();
                loop {
                    self.ws();
                    if self.bytes.get(self.pos) == Some(&close) {
                        self.pos += 1;
                        break;
                    }
                    values.push(self.value(depth + 1)?);
                    self.ws();
                    if self.bytes.get(self.pos) == Some(&b',') {
                        self.pos += 1;
                    } else {
                        ensure!(
                            self.bytes.get(self.pos) == Some(&close),
                            "Missing NPY header delimiter"
                        );
                    }
                }
                Ok(Literal::Seq(values))
            }
            b'{' => {
                self.pos += 1;
                let mut pairs = Vec::new();
                loop {
                    self.ws();
                    if self.bytes.get(self.pos) == Some(&b'}') {
                        self.pos += 1;
                        break;
                    }
                    let key = self.string()?;
                    self.ws();
                    ensure!(
                        self.bytes.get(self.pos) == Some(&b':'),
                        "Missing NPY header colon"
                    );
                    self.pos += 1;
                    pairs.push((key, self.value(depth + 1)?));
                    self.ws();
                    if self.bytes.get(self.pos) == Some(&b',') {
                        self.pos += 1;
                    } else {
                        ensure!(
                            self.bytes.get(self.pos) == Some(&b'}'),
                            "Missing NPY header delimiter"
                        );
                    }
                }
                Ok(Literal::Dict(pairs))
            }
            b'0'..=b'9' => {
                let start = self.pos;
                while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                    self.pos += 1;
                }
                Ok(Literal::Int(
                    std::str::from_utf8(&self.bytes[start..self.pos])?.parse()?,
                ))
            }
            _ => {
                for (token, value) in [
                    ("True", Literal::Bool(true)),
                    ("False", Literal::Bool(false)),
                    ("None", Literal::None),
                ] {
                    if self.bytes[self.pos..].starts_with(token.as_bytes()) {
                        self.pos += token.len();
                        return Ok(value);
                    }
                }
                bail!("Unsupported NPY header literal")
            }
        }
    }
    fn string(&mut self) -> Result<String> {
        self.ws();
        let quote = *self.bytes.get(self.pos).context("Missing string")?;
        ensure!(quote == b'\'' || quote == b'"', "Invalid string");
        self.pos += 1;
        let mut out = String::new();
        let mut plain = Vec::new();
        loop {
            let c = *self.bytes.get(self.pos).context("Unterminated string")?;
            self.pos += 1;
            if c == quote {
                out.push_str(&String::from_utf8(plain)?);
                return Ok(out);
            }
            if c != b'\\' {
                plain.push(c);
                continue;
            }
            out.push_str(&String::from_utf8(std::mem::take(&mut plain))?);
            let c = *self
                .bytes
                .get(self.pos)
                .context("Incomplete escape sequence")?;
            self.pos += 1;
            match c {
                b'x' | b'u' | b'U' => {
                    let len = match c {
                        b'x' => 2,
                        b'u' => 4,
                        _ => 8,
                    };
                    let hex = self
                        .bytes
                        .get(self.pos..self.pos + len)
                        .context("Incomplete escape sequence")?;
                    let code = u32::from_str_radix(std::str::from_utf8(hex)?, 16)?;
                    out.push(char::from_u32(code).context("Invalid Unicode")?);
                    self.pos += len;
                }
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                b'\\' => out.push('\\'),
                b'\'' => out.push('\''),
                b'"' => out.push('"'),
                _ => bail!("Unsupported string escape"),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct DType {
    pub label: String,
    pub kind: char,
    pub size: usize,
    pub big_endian: bool,
    pub fields: Vec<(String, usize, DType)>,
}
impl DType {
    fn parse(value: &Literal) -> Result<Self> {
        if let Literal::Seq(fields) = value {
            let mut dtype = Self {
                label: "structured".into(),
                kind: 'V',
                size: 0,
                big_endian: false,
                fields: Vec::new(),
            };
            for field in fields {
                let Literal::Seq(parts) = field else {
                    bail!("Invalid structured dtype");
                };
                ensure!(parts.len() == 2 || parts.len() == 3, "Invalid dtype field");
                let name = match &parts[0] {
                    Literal::Str(s) => s.clone(),
                    Literal::Seq(s) => match s.last() {
                        Some(Literal::Str(s)) => s.clone(),
                        _ => bail!("Invalid field name"),
                    },
                    _ => bail!("Invalid field name"),
                };
                let mut child = Self::parse(&parts[1])?;
                if parts.len() == 3 {
                    let shape = shape_from(&parts[2])?;
                    child.size = child
                        .size
                        .checked_mul(product(&shape)?)
                        .context("dtype size overflow")?;
                    child.label = format!("{} {shape:?}", child.label);
                    child.kind = 'V';
                    child.fields.clear();
                }
                ensure!(
                    child.kind != 'O',
                    "Object arrays require pickle, which this viewer does not execute. Save metadata as Unicode / JSON strings."
                );
                let offset = dtype.size;
                dtype.size = offset
                    .checked_add(child.size)
                    .context("dtype size overflow")?;
                dtype.fields.push((name, offset, child));
            }
            return Ok(dtype);
        }
        let Literal::Str(label) = value else {
            bail!("Invalid dtype");
        };
        let mut chars = label.chars();
        let order = chars.next().context("Empty dtype")?;
        ensure!(
            matches!(order, '<' | '>' | '|' | '='),
            "Unknown dtype byte order"
        );
        let kind = chars.next().context("Missing dtype kind")?;
        ensure!(
            kind != 'O',
            "Object arrays require pickle, which this viewer does not execute. Save metadata as Unicode / JSON strings."
        );
        let suffix: String = chars.collect();
        let len: usize = suffix
            .split('[')
            .next()
            .unwrap_or("")
            .parse()
            .context("Invalid dtype length")?;
        let size = len
            .checked_mul(if kind == 'U' { 4 } else { 1 })
            .context("dtype size overflow")?;
        Ok(Self {
            label: label.clone(),
            kind,
            size,
            big_endian: order == '>' || (order == '=' && cfg!(target_endian = "big")),
            fields: Vec::new(),
        })
    }
    pub fn numeric(&self) -> bool {
        matches!(
            (self.kind, self.size),
            ('b' | '?', 1) | ('i' | 'u', 1 | 2 | 4 | 8) | ('f', 2 | 4 | 8) | ('c', 8 | 16)
        )
    }
    pub fn text(&self) -> bool {
        matches!(self.kind, 'U' | 'S')
    }
    fn uint(&self, bytes: &[u8]) -> u64 {
        if self.big_endian {
            bytes.iter().fold(0, |a, b| (a << 8) | u64::from(*b))
        } else {
            bytes.iter().rev().fold(0, |a, b| (a << 8) | u64::from(*b))
        }
    }
    fn real(&self, bytes: &[u8]) -> Option<f64> {
        let raw = self.uint(bytes);
        Some(match (self.kind, bytes.len()) {
            ('b' | '?', 1) => f64::from(raw != 0),
            ('u', 1 | 2 | 4 | 8) => raw as f64,
            ('i', 1 | 2 | 4 | 8) => {
                let shift = 64 - bytes.len() * 8;
                ((raw << shift) as i64 >> shift) as f64
            }
            ('f' | 'c', 2) => half::f16::from_bits(raw as u16).to_f64(),
            ('f' | 'c', 4) => f32::from_bits(raw as u32) as f64,
            ('f' | 'c', 8) => f64::from_bits(raw),
            _ => return None,
        })
    }
    pub fn display(&self, bytes: &[u8]) -> String {
        match self.kind {
            'U' => {
                let mut text = String::new();
                for chunk in bytes.chunks_exact(4) {
                    let code = self.uint(chunk) as u32;
                    text.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                }
                text.trim_end_matches('\0').to_owned()
            }
            'S' => String::from_utf8_lossy(bytes)
                .trim_end_matches('\0')
                .to_owned(),
            'b' | '?' => if bytes.first() == Some(&0) {
                "False"
            } else {
                "True"
            }
            .into(),
            'u' if bytes.len() <= 8 => self.uint(bytes).to_string(),
            'i' if !bytes.is_empty() && bytes.len() <= 8 => {
                let shift = 64 - bytes.len() * 8;
                ((self.uint(bytes) << shift) as i64 >> shift).to_string()
            }
            'c' if matches!(bytes.len(), 8 | 16) => {
                let n = bytes.len() / 2;
                format!(
                    "{} {:+}j",
                    self.real(&bytes[..n]).unwrap(),
                    self.real(&bytes[n..]).unwrap()
                )
            }
            'V' if !self.fields.is_empty() => {
                let values: Vec<_> = self
                    .fields
                    .iter()
                    .filter(|(name, _, _)| !name.is_empty())
                    .map(|(name, offset, dtype)| {
                        format!(
                            "{name}: {}",
                            dtype.display(&bytes[*offset..*offset + dtype.size])
                        )
                    })
                    .collect();
                format!("{{{}}}", values.join(", "))
            }
            _ => match self.real(bytes) {
                Some(v) => format!("{v}"),
                None => format!(
                    "{} [0x{}]",
                    self.label,
                    bytes
                        .iter()
                        .take(24)
                        .map(|v| format!("{v:02x}"))
                        .collect::<String>()
                ),
            },
        }
    }
}

fn product(shape: &[usize]) -> Result<usize> {
    shape.iter().try_fold(1usize, |a, b| {
        a.checked_mul(*b).context("Array size overflow")
    })
}
fn shape_from(value: &Literal) -> Result<Vec<usize>> {
    let Literal::Seq(dims) = value else {
        bail!("Invalid shape");
    };
    ensure!(dims.len() <= 64, "Too many dimensions");
    dims.iter()
        .map(|v| match v {
            Literal::Int(n) => Ok(*n),
            _ => bail!("Invalid shape dimension"),
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Header {
    pub shape: Vec<usize>,
    pub dtype: DType,
    pub fortran: bool,
    pub offset: usize,
    pub count: usize,
    pub nbytes: usize,
}
impl Header {
    pub fn read(reader: &mut impl Read) -> Result<Self> {
        let mut magic = [0; 8];
        reader.read_exact(&mut magic)?;
        ensure!(&magic[..6] == b"\x93NUMPY", "Invalid NPY signature");
        ensure!(
            matches!(magic[6], 1..=3) && magic[7] == 0,
            "Unsupported NPY version"
        );
        let width = if magic[6] == 1 { 2 } else { 4 };
        let mut size = [0u8; 4];
        reader.read_exact(&mut size[..width])?;
        let len = u32::from_le_bytes(size) as usize;
        ensure!(len <= 1024 * 1024, "NPY header exceeds 1 MiB");
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes)?;
        let header = if magic[6] == 3 {
            String::from_utf8(bytes)?
        } else {
            bytes.into_iter().map(char::from).collect()
        };
        let mut parser = Parser {
            bytes: header.as_bytes(),
            pos: 0,
        };
        let Literal::Dict(pairs) = parser.value(0)? else {
            bail!("NPY header is not a dictionary");
        };
        parser.ws();
        ensure!(
            parser.pos == parser.bytes.len(),
            "Trailing content in NPY header"
        );
        let get = |key: &str| -> Result<&Literal> {
            ensure!(
                pairs.iter().filter(|(k, _)| k == key).count() == 1,
                "Missing or duplicate NPY header field: {key}"
            );
            Ok(&pairs.iter().find(|(k, _)| k == key).unwrap().1)
        };
        let shape = shape_from(get("shape")?)?;
        let dtype = DType::parse(get("descr")?)?;
        let Literal::Bool(fortran) = get("fortran_order")? else {
            bail!("Invalid fortran_order");
        };
        let count = product(&shape)?;
        let nbytes = count
            .checked_mul(dtype.size)
            .context("Array size overflow")?;
        Ok(Self {
            shape,
            dtype,
            fortran: *fortran,
            offset: 8 + width + len,
            count,
            nbytes,
        })
    }
}

pub struct Array {
    pub header: Header,
    pub name: String,
    map: Mmap,
    // Retain the anonymous extracted file until the last mapped view is dropped.
    _temporary: Option<File>,
}
impl Array {
    pub fn load(
        catalog: &Catalog,
        index: usize,
        generation: &AtomicU64,
        ticket: u64,
        mut progress: impl FnMut(u64, u64),
    ) -> Result<Self> {
        let entry = catalog
            .entries
            .get(index)
            .context("Array index is no longer valid")?;
        let (file, temporary) = if let Some(zip_index) = entry.zip_index {
            let mut archive = zip::ZipArchive::new(File::open(&catalog.path)?)?;
            let mut member = archive.by_index(zip_index)?;
            // Validate before allocating disk space, including object/pickle rejection.
            let header = Header::read(&mut member)?;
            ensure!(
                header.offset as u64 + header.nbytes as u64 == entry.bytes,
                "NPY payload size does not match the header"
            );
            drop(member);
            let mut member = archive.by_index(zip_index)?;
            let mut temp =
                tempfile::tempfile().context("Cannot create a temporary mapping file")?;
            let mut buffer = vec![0u8; 1024 * 1024];
            let mut done = 0u64;
            loop {
                ensure!(
                    generation.load(Ordering::Relaxed) == ticket,
                    "Operation cancelled"
                );
                let n = member
                    .read(&mut buffer)
                    .context("NPZ decompression failed (corrupt file or CRC mismatch)")?;
                if n == 0 {
                    break;
                }
                temp.write_all(&buffer[..n])
                    .context("Temporary file write failed (check available disk space)")?;
                done += n as u64;
                progress(done, entry.bytes);
            }
            ensure!(done == entry.bytes, "Incomplete NPZ data");
            temp.rewind()?;
            (temp, true)
        } else {
            (File::open(&catalog.path)?, false)
        };
        let header = Header::read(&mut BufReader::new(&file))?;
        ensure!(
            header.offset as u64 + header.nbytes as u64 == file.metadata()?.len(),
            "NPY payload size does not match the header"
        );
        // SAFETY: Read-only map, file ownership retained for extracted members.
        // The application never writes/truncates a mapped file.
        let map = unsafe { MmapOptions::new().map(&file)? };
        Ok(Self {
            header,
            name: entry.name.clone(),
            map,
            _temporary: temporary.then_some(file),
        })
    }
    pub fn offset_of(&self, indices: &[usize]) -> Option<usize> {
        if indices.len() != self.header.shape.len() {
            return None;
        }
        let mut offset = 0usize;
        let mut stride = 1usize;
        let axes: Box<dyn Iterator<Item = usize>> = if self.header.fortran {
            Box::new(0..indices.len())
        } else {
            Box::new((0..indices.len()).rev())
        };
        for axis in axes {
            if indices[axis] >= self.header.shape[axis] {
                return None;
            }
            offset += indices[axis] * stride;
            stride *= self.header.shape[axis];
        }
        Some(offset)
    }
    fn bytes(&self, offset: usize) -> Option<&[u8]> {
        if offset >= self.header.count {
            return None;
        }
        let start = self.header.offset + offset * self.header.dtype.size;
        self.map.get(start..start + self.header.dtype.size)
    }
    pub fn display(&self, indices: &[usize]) -> String {
        self.offset_of(indices)
            .and_then(|o| self.bytes(o))
            .map(|b| self.header.dtype.display(b))
            .unwrap_or_default()
    }
    pub fn number_at_offset(&self, offset: usize, component: Component) -> Option<f64> {
        let bytes = self.bytes(offset)?;
        let dtype = &self.header.dtype;
        if !dtype.numeric() {
            return None;
        }
        if dtype.kind != 'c' {
            return dtype.real(bytes);
        }
        let half = bytes.len() / 2;
        let real = dtype.real(&bytes[..half])?;
        let imag = dtype.real(&bytes[half..])?;
        Some(match component {
            Component::Real => real,
            Component::Imag => imag,
            Component::Magnitude => real.hypot(imag),
            Component::Phase => imag.atan2(real),
        })
    }
    /// Flat logical C-order index inside an arbitrary leading-index slice.
    pub fn slice_offset(&self, prefix: &[usize], mut flat: usize) -> Option<usize> {
        if prefix.len() > self.header.shape.len()
            || prefix.iter().zip(&self.header.shape).any(|(i, n)| i >= n)
        {
            return None;
        }
        let shape = &self.header.shape[prefix.len()..];
        let count = shape.iter().product::<usize>();
        if flat >= count {
            return None;
        }
        if !self.header.fortran {
            let mut base = 0;
            for (&i, &n) in prefix.iter().zip(&self.header.shape) {
                base = base * n + i;
            }
            return Some(base * count + flat);
        }
        let mut base = 0;
        let mut stride = 1;
        for (&i, &n) in prefix.iter().zip(&self.header.shape) {
            base += i * stride;
            stride *= n;
        }
        // Convert C-order suffix coordinates to physical Fortran-order strides.
        let mut divisor = count;
        for &n in shape {
            divisor /= n;
            let index = flat / divisor;
            flat %= divisor;
            base += index * stride;
            stride *= n;
        }
        Some(base)
    }
    pub fn slice_number(&self, prefix: &[usize], flat: usize, component: Component) -> Option<f64> {
        self.number_at_offset(self.slice_offset(prefix, flat)?, component)
    }
    pub fn slice_len(&self, prefix: &[usize]) -> usize {
        self.header.shape[prefix.len()..].iter().product()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Component {
    #[default]
    Real,
    Imag,
    Magnitude,
    Phase,
}

pub fn size_label(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
