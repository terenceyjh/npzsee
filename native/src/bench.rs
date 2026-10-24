// SPDX-License-Identifier: GPL-3.0-or-later
//! Headless correctness/performance probe using the very same data/analysis path as the GUI.
use anyhow::{Context, Result};
use npzsee::{
    analysis::{Envelope, Histogram, Layout, Series},
    data::{Array, Catalog, Component},
};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(
        args.next()
            .context("usage: npzsee-bench FILE [KEY] [--inspect]")?,
    );
    let key = args.next().unwrap_or_else(|| "voltage".into());
    let inspect = args.any(|a| a == "--inspect");
    let start = Instant::now();
    let catalog = Catalog::open(&path)?;
    let index_ms = start.elapsed().as_secs_f64() * 1000.;
    let index = catalog
        .entries
        .iter()
        .position(|e| e.name == key)
        .context("key not found")?;
    let generation = AtomicU64::new(1);
    let start = Instant::now();
    let array = Arc::new(Array::load(&catalog, index, &generation, 1, |_, _| {})?);
    let load_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let mut values = Vec::new();
    for i in 0..array.header.count.min(if inspect { 2048 } else { 10 }) {
        let offset = array.slice_offset(&[], i).unwrap();
        // Build C-order indices only for textual inspection, off the rendering path.
        let mut rem = i;
        let mut indices = vec![0; array.header.shape.len()];
        for axis in (0..indices.len()).rev() {
            indices[axis] = rem % array.header.shape[axis];
            rem /= array.header.shape[axis];
        }
        values.push(serde_json::json!({"offset":offset,"value":array.display(&indices)}));
    }
    let table_ms = start.elapsed().as_secs_f64() * 1000.;
    let mut result = serde_json::json!({"arrays":catalog.entries.len(), "index_ms":index_ms,"load_ms":load_ms,"table_ms":table_ms,"shape":array.header.shape,"dtype":array.header.dtype.label,"values":values});
    if array.header.dtype.numeric() && !inspect {
        let start = Instant::now();
        let envelope = Envelope::build(
            Series::new(array.clone(), Vec::new(), Component::Real, Layout::Row, 0),
            &generation,
            1,
        )?;
        result["envelope_ms"] = (start.elapsed().as_secs_f64() * 1000.).into();
        let start = Instant::now();
        let points = envelope.points(envelope.x_range, 1400);
        result["overview_ms"] = (start.elapsed().as_secs_f64() * 1000.).into();
        result["drawn_points"] = points.len().into();
        let start = Instant::now();
        let zoom = envelope.points([100_000., 110_000.], 1400);
        result["zoom_ms"] = (start.elapsed().as_secs_f64() * 1000.).into();
        result["zoom_points"] = zoom.len().into();
        let start = Instant::now();
        let quick = Histogram::build(&array, &[], Component::Real, 128, false, &generation, 1)?;
        result["hist_preview_ms"] = (start.elapsed().as_secs_f64() * 1000.).into();
        result["hist_preview_samples"] = quick.sampled.into();
        let start = Instant::now();
        let exact = Histogram::build(&array, &[], Component::Real, 128, true, &generation, 1)?;
        result["hist_exact_ms"] = (start.elapsed().as_secs_f64() * 1000.).into();
        result["exact"] = serde_json::json!({"count":exact.count,"nonfinite":exact.nonfinite,"mean":exact.mean,"std":exact.std,"range":exact.range,"counts":exact.counts});
    }
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
