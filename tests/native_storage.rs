// SPDX-License-Identifier: GPL-3.0-or-later
use npzsee::{
    analysis::{Envelope, Histogram, Layout, Series},
    data::{Array, Catalog, Component, Header},
};
use std::{
    io::{Cursor, Write},
    sync::{Arc, atomic::AtomicU64},
};

fn npy(descr: &str, shape: &str, fortran: bool, data: &[u8], version: u8) -> Vec<u8> {
    let header = format!(
        "{{'descr': {descr}, 'fortran_order': {}, 'shape': {shape}, }}\n",
        if fortran { "True" } else { "False" }
    );
    let mut bytes = b"\x93NUMPY".to_vec();
    bytes.extend([version, 0]);
    if version == 1 {
        bytes.extend((header.len() as u16).to_le_bytes());
    } else {
        bytes.extend((header.len() as u32).to_le_bytes());
    }
    bytes.extend(header.as_bytes());
    bytes.extend(data);
    bytes
}
fn load(bytes: &[u8]) -> Arc<Array> {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    let catalog = Catalog::open(file.path()).unwrap();
    Arc::new(Array::load(&catalog, 0, &AtomicU64::new(1), 1, |_, _| {}).unwrap())
}
fn vector(values: &[f64]) -> Arc<Array> {
    load(&npy(
        "'<f8'",
        &format!("({},)", values.len()),
        false,
        &values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
        1,
    ))
}
#[test]
fn npy_versions_and_scalars() {
    for version in [1, 2, 3] {
        let a = load(&npy(
            "'<i8'",
            "()",
            false,
            &(-123i64).to_le_bytes(),
            version,
        ));
        assert_eq!(a.display(&[]), "-123");
        assert_eq!(a.header.count, 1);
    }
}
#[test]
fn big_endian_float_and_integer() {
    let a = load(&npy(
        "'>f8'",
        "(2,)",
        false,
        &[1.25f64.to_be_bytes(), (-7.5f64).to_be_bytes()].concat(),
        1,
    ));
    assert_eq!(a.slice_number(&[], 0, Component::Real), Some(1.25));
    assert_eq!(a.display(&[1]), "-7.5");
    let b = load(&npy(
        "'>i2'",
        "(2,)",
        false,
        &[(-32768i16).to_be_bytes(), 32767i16.to_be_bytes()].concat(),
        1,
    ));
    assert_eq!(b.display(&[0]), "-32768");
    assert_eq!(b.display(&[1]), "32767");
}
#[test]
fn exact_large_integer_display() {
    let a = load(&npy("'<u8'", "()", false, &u64::MAX.to_le_bytes(), 1));
    assert_eq!(a.display(&[]), u64::MAX.to_string());
}
#[test]
fn unicode_and_embedded_nul() {
    for endian in [false, true] {
        let bytes: Vec<_> = ['中', '文', '\0', '🙂', '\0']
            .into_iter()
            .flat_map(|c| {
                if endian {
                    (c as u32).to_be_bytes()
                } else {
                    (c as u32).to_le_bytes()
                }
            })
            .collect();
        let a = load(&npy(
            if endian { "'>U5'" } else { "'<U5'" },
            "()",
            false,
            &bytes,
            3,
        ));
        assert_eq!(a.display(&[]), "中文\0🙂");
    }
}
#[test]
fn byte_strings_and_bool() {
    assert_eq!(
        load(&npy("'|S5'", "()", false, b"abc\0\0", 1)).display(&[]),
        "abc"
    );
    let a = load(&npy("'|b1'", "(2,)", false, &[0, 1], 1));
    assert_eq!(a.display(&[0]), "False");
    assert_eq!(a.display(&[1]), "True");
}
#[test]
fn float16_and_complex_components() {
    let a = load(&npy(
        "'<f2'",
        "()",
        false,
        &half::f16::from_f32(1.5).to_bits().to_le_bytes(),
        1,
    ));
    assert_eq!(a.slice_number(&[], 0, Component::Real), Some(1.5));
    let a = load(&npy(
        "'<c16'",
        "()",
        false,
        &[3f64.to_le_bytes(), 4f64.to_le_bytes()].concat(),
        1,
    ));
    assert_eq!(a.slice_number(&[], 0, Component::Magnitude), Some(5.));
    assert_eq!(a.slice_number(&[], 0, Component::Imag), Some(4.));
}
#[test]
fn c_and_fortran_hierarchical_slices() {
    let c = load(&npy(
        "'<i4'",
        "(2, 3, 4)",
        false,
        &(0i32..24).flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
        1,
    ));
    let mut fbytes = Vec::new();
    for k in 0i32..4 {
        for j in 0i32..3 {
            for i in 0i32..2 {
                fbytes.extend((i * 12 + j * 4 + k).to_le_bytes());
            }
        }
    }
    let f = load(&npy("'<i4'", "(2, 3, 4)", true, &fbytes, 1));
    for a in [&c, &f] {
        assert_eq!(a.display(&[1, 2, 3]), "23");
        for i in 0..12 {
            assert_eq!(
                a.slice_number(&[1], i, Component::Real),
                Some((12 + i) as f64)
            );
        }
        assert_eq!(a.slice_number(&[1, 2], 1, Component::Real), Some(21.));
        assert!(a.offset_of(&[2, 0, 0]).is_none());
    }
}
#[test]
fn empty_and_zero_width_arrays() {
    let a = load(&npy("'<f8'", "(2, 0, 3)", false, &[], 1));
    assert_eq!(a.header.count, 0);
    assert!(a.slice_offset(&[], 0).is_none());
    let a = load(&npy("'|S0'", "(1,)", false, &[], 1));
    assert_eq!(a.display(&[0]), "");
}
#[test]
fn structured_fields() {
    let a = load(&npy(
        "[('温度', '<f8'), ('count', '>i2')]",
        "()",
        false,
        &[2.5f64.to_le_bytes().to_vec(), 42i16.to_be_bytes().to_vec()].concat(),
        3,
    ));
    assert_eq!(a.display(&[]), "{温度: 2.5, count: 42}");
}
#[test]
fn malicious_or_overflowing_headers_are_rejected() {
    for bytes in [
        npy("'|O'", "()", false, &[], 1),
        npy("'<f8'", "(18446744073709551615, 2)", false, &[], 1),
        npy("__import__('os')", "()", false, &[], 1),
    ] {
        assert!(Header::read(&mut Cursor::new(bytes)).is_err());
    }
    assert!(Header::read(&mut Cursor::new(b"not an npy")).is_err());
}
#[test]
fn truncated_payload_is_rejected() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(&npy("'<f8'", "(100,)", false, &[0; 8], 1))
        .unwrap();
    let c = Catalog::open(f.path()).unwrap();
    assert!(Array::load(&c, 0, &AtomicU64::new(1), 1, |_, _| {}).is_err());
}
#[test]
fn catalog_does_not_read_or_validate_member_payloads() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    {
        let mut zip = zip::ZipWriter::new(f.as_file_mut());
        zip.start_file("bad.npy", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"invalid header").unwrap();
        zip.start_file("notes.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"ignore").unwrap();
        zip.finish().unwrap();
    }
    let c = Catalog::open(f.path()).unwrap();
    assert_eq!(c.entries.len(), 1);
    assert_eq!(c.entries[0].name, "bad");
    assert!(Array::load(&c, 0, &AtomicU64::new(1), 1, |_, _| {}).is_err());
}
#[test]
fn compressed_member_load_and_cancellation() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    {
        let mut zip = zip::ZipWriter::new(f.as_file_mut());
        zip.start_file(
            "signal.npy",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(&npy(
            "'<i4'",
            "(3,)",
            false,
            &[5i32.to_le_bytes(), 6i32.to_le_bytes(), 7i32.to_le_bytes()].concat(),
            1,
        ))
        .unwrap();
        zip.finish().unwrap();
    }
    let c = Catalog::open(f.path()).unwrap();
    assert!(Array::load(&c, 0, &AtomicU64::new(2), 1, |_, _| {}).is_err());
    let a = Array::load(&c, 0, &AtomicU64::new(1), 1, |_, _| {}).unwrap();
    assert_eq!(a.display(&[2]), "7");
}
#[test]
fn envelope_preserves_narrow_spikes_and_zoom_values() {
    let mut values = vec![0.; 200_000];
    values[12345] = 100.;
    values[15678] = -200.;
    let a = vector(&values);
    let env = Envelope::build(
        Series::new(a, vec![], Component::Real, Layout::Row, 0),
        &AtomicU64::new(1),
        1,
    )
    .unwrap();
    let points = env.points([0., 200_000.], 100);
    assert!(points.contains(&[12345., 100.]));
    assert!(points.contains(&[15678., -200.]));
    assert!(points.len() <= 204);
    let zoom = env.points([12340., 12350.], 100);
    assert!(zoom.contains(&[12345., 100.]));
    assert!(zoom.iter().all(|p| p[0] >= 12340. && p[0] <= 12351.));
}
#[test]
fn xy_and_row_layouts() {
    let a = load(&npy(
        "'<f8'",
        "(3, 2)",
        false,
        &[1., 10., 2., 20., 3., 30.]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect::<Vec<_>>(),
        1,
    ));
    let s = Series::new(a.clone(), vec![], Component::Real, Layout::XyColumns, 0);
    assert_eq!(s.point(2), [3., 30.]);
    let row = Series::new(a, vec![], Component::Real, Layout::Row, 1);
    assert_eq!(row.len, 2);
    assert_eq!(row.point(0), [0., 2.]);
}
#[test]
fn histograms_count_finite_values_and_statistics() {
    let a = vector(&[1., 2., 3., 4., f64::NAN, f64::INFINITY]);
    let hist = Histogram::build(&a, &[], Component::Real, 8, true, &AtomicU64::new(1), 1).unwrap();
    assert_eq!(hist.count, 4);
    assert_eq!(hist.nonfinite, 2);
    assert_eq!(hist.counts.iter().sum::<u64>(), 4);
    assert_eq!(hist.mean, 2.5);
    assert!((hist.std - 1.25f64.sqrt()).abs() < 1e-12);
}
#[test]
fn histogram_empty_constant_and_sampling_are_explicit() {
    let a = vector(&[]);
    let h = Histogram::build(&a, &[], Component::Real, 8, true, &AtomicU64::new(1), 1).unwrap();
    assert_eq!(h.count, 0);
    let a = vector(&[2.; 500_001]);
    let h = Histogram::build(&a, &[], Component::Real, 8, false, &AtomicU64::new(1), 1).unwrap();
    assert!(h.sampled <= 250_000);
    assert!(h.sampled < h.total);
    assert_eq!(h.counts.iter().sum::<u64>(), h.count);
    assert_eq!(h.mean, 2.);
    assert_eq!(h.data_range, [2., 2.]);
    assert!(h.range[0] < 2. && h.range[1] > 2.);
}
#[test]
fn zip_crc_corruption_is_reported() {
    use std::{
        fs::File,
        io::{Read, Seek, SeekFrom},
    };
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let bytes = npy("'<i4'", "(1,)", false, &42i32.to_le_bytes(), 1);
    {
        let mut zip = zip::ZipWriter::new(file.as_file_mut());
        zip.start_file(
            "value.npy",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(&bytes).unwrap();
        zip.finish().unwrap();
    }
    let offset = {
        let mut zip = zip::ZipArchive::new(File::open(file.path()).unwrap()).unwrap();
        zip.by_index(0).unwrap().data_start().unwrap()
    };
    file.as_file_mut()
        .seek(SeekFrom::Start(offset + bytes.len() as u64 - 1))
        .unwrap();
    let mut last = [0];
    file.as_file_mut().read_exact(&mut last).unwrap();
    last[0] ^= 0xff;
    file.as_file_mut().seek(SeekFrom::Current(-1)).unwrap();
    file.write_all(&last).unwrap();
    let catalog = Catalog::open(file.path()).unwrap();
    assert!(Array::load(&catalog, 0, &AtomicU64::new(1), 1, |_, _| {}).is_err());
}

#[test]
fn npz_keys_strip_exactly_one_extension() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    {
        let mut zip = zip::ZipWriter::new(f.as_file_mut());
        zip.start_file(
            "nested/key.npy.npy",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(&npy("'<i4'", "()", false, &42i32.to_le_bytes(), 1))
            .unwrap();
        zip.finish().unwrap();
    }
    let catalog = Catalog::open(f.path()).unwrap();
    assert_eq!(catalog.entries[0].name, "nested/key.npy");
}

#[test]
fn analysis_cancels_stale_jobs() {
    let a = vector(&[1., 2., 3.]);
    assert!(
        Envelope::build(
            Series::new(a.clone(), vec![], Component::Real, Layout::Row, 0),
            &AtomicU64::new(2),
            1
        )
        .is_err()
    );
    assert!(Histogram::build(&a, &[], Component::Real, 8, true, &AtomicU64::new(2), 1).is_err());
}
