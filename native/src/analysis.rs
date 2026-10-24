// SPDX-License-Identifier: GPL-3.0-or-later
use crate::data::{Array, Component};
use anyhow::{Result, ensure};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Row,
    XyRows,
    XyColumns,
}

#[derive(Clone)]
pub struct Series {
    pub array: Arc<Array>,
    pub prefix: Vec<usize>,
    pub component: Component,
    pub layout: Layout,
    pub row: usize,
    pub len: usize,
}
impl Series {
    pub fn new(
        array: Arc<Array>,
        mut prefix: Vec<usize>,
        component: Component,
        layout: Layout,
        row: usize,
    ) -> Self {
        let shape = &array.header.shape[prefix.len()..];
        let len = match (layout, shape) {
            (Layout::XyRows, [2, n]) | (Layout::XyColumns, [n, 2]) => *n,
            (_, [rows, n]) => {
                let rows = *rows;
                let n = *n;
                prefix.push(row.min(rows.saturating_sub(1)));
                if rows == 0 { 0 } else { n }
            }
            (_, [n]) => *n,
            (_, []) => 1,
            _ => 0,
        };
        Self {
            array,
            prefix,
            component,
            layout,
            row,
            len,
        }
    }
    pub fn point(&self, i: usize) -> [f64; 2] {
        let (x, y) = match self.layout {
            Layout::XyRows => (
                self.array.slice_number(&self.prefix, i, self.component),
                self.array
                    .slice_number(&self.prefix, self.len + i, self.component),
            ),
            Layout::XyColumns => (
                self.array.slice_number(&self.prefix, i * 2, self.component),
                self.array
                    .slice_number(&self.prefix, i * 2 + 1, self.component),
            ),
            Layout::Row => (
                Some(i as f64),
                self.array.slice_number(&self.prefix, i, self.component),
            ),
        };
        [x.unwrap_or(f64::NAN), y.unwrap_or(f64::NAN)]
    }
}

#[derive(Clone, Debug)]
pub struct Bucket {
    pub min: [f64; 2],
    pub max: [f64; 2],
    pub min_i: usize,
    pub max_i: usize,
}
impl Bucket {
    fn new() -> Self {
        Self {
            min: [f64::NAN, f64::INFINITY],
            max: [f64::NAN, f64::NEG_INFINITY],
            min_i: 0,
            max_i: 0,
        }
    }
    fn push(&mut self, point: [f64; 2], i: usize) {
        if !point[0].is_finite() || !point[1].is_finite() {
            return;
        }
        if point[1] < self.min[1] {
            self.min = point;
            self.min_i = i;
        }
        if point[1] > self.max[1] {
            self.max = point;
            self.max_i = i;
        }
    }
    fn emit(&self, out: &mut Vec<[f64; 2]>) {
        if !self.min[0].is_finite() {
            out.push([f64::NAN; 2]);
        } else if self.min_i <= self.max_i {
            out.push(self.min);
            if self.max_i != self.min_i {
                out.push(self.max);
            }
        } else {
            out.push(self.max);
            out.push(self.min);
        }
    }
}

pub struct Envelope {
    pub series: Series,
    pub buckets: Vec<Bucket>,
    pub block: usize,
    pub x_range: [f64; 2],
    pub y_range: [f64; 2],
    pub finite: u64,
}
impl Envelope {
    pub fn build(series: Series, generation: &AtomicU64, ticket: u64) -> Result<Self> {
        let block = series.len.div_ceil(65_536).max(64);
        let mut buckets = Vec::with_capacity(series.len.div_ceil(block));
        let mut x_range = [f64::INFINITY, f64::NEG_INFINITY];
        let mut y_range = x_range;
        let mut finite = 0;
        for start in (0..series.len).step_by(block) {
            ensure!(
                generation.load(Ordering::Relaxed) == ticket,
                "Operation cancelled"
            );
            let mut bucket = Bucket::new();
            for i in start..(start + block).min(series.len) {
                let p = series.point(i);
                if p[0].is_finite() && p[1].is_finite() {
                    x_range[0] = x_range[0].min(p[0]);
                    x_range[1] = x_range[1].max(p[0]);
                    y_range[0] = y_range[0].min(p[1]);
                    y_range[1] = y_range[1].max(p[1]);
                    finite += 1;
                }
                bucket.push(p, i);
            }
            buckets.push(bucket);
        }
        Ok(Self {
            series,
            buckets,
            block,
            x_range,
            y_range,
            finite,
        })
    }
    /// At most ~2 points per screen column, preserving extrema and their order.
    /// For index plots, zoom re-reads only the visible portion of the original data.
    pub fn points(&self, range: [f64; 2], columns: usize) -> Vec<[f64; 2]> {
        let columns = columns.clamp(64, 4096);
        let (start, end) = if self.series.layout == Layout::Row {
            (
                (range[0].floor().max(0.) as usize).min(self.series.len),
                (range[1].ceil().max(0.) as usize)
                    .saturating_add(1)
                    .min(self.series.len),
            )
        } else {
            (0, self.series.len)
        };
        if start >= end {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(columns * 2 + 4);
        let step = (end - start).div_ceil(columns).max(1);
        if step < self.block {
            for a in (start..end).step_by(step) {
                let mut bucket = Bucket::new();
                for i in a..(a + step).min(end) {
                    bucket.push(self.series.point(i), i);
                }
                bucket.emit(&mut out);
            }
        } else {
            let first = start / self.block;
            let last = end.div_ceil(self.block);
            let stride = (last - first).div_ceil(columns).max(1);
            for a in (first..last).step_by(stride) {
                let mut merged = Bucket::new();
                for b in a..(a + stride).min(last) {
                    let lo = b * self.block;
                    let hi = (lo + self.block).min(self.series.len);
                    if lo < start || hi > end {
                        for i in lo.max(start)..hi.min(end) {
                            merged.push(self.series.point(i), i);
                        }
                    } else {
                        let source = &self.buckets[b];
                        merged.push(source.min, source.min_i);
                        merged.push(source.max, source.max_i);
                    }
                }
                merged.emit(&mut out);
            }
        }
        out
    }
}

#[derive(Clone, Debug)]
pub struct Histogram {
    pub counts: Vec<u64>,
    pub range: [f64; 2],
    pub data_range: [f64; 2],
    pub count: u64,
    pub sampled: usize,
    pub total: usize,
    pub nonfinite: u64,
    pub mean: f64,
    pub std: f64,
}
impl Histogram {
    /// Quick mode is explicitly labelled as a deterministic evenly spaced sample.
    /// Exact mode scans all values off the UI thread in two bounded-memory passes.
    pub fn build(
        array: &Array,
        prefix: &[usize],
        component: Component,
        bins: usize,
        exact: bool,
        generation: &AtomicU64,
        ticket: u64,
    ) -> Result<Self> {
        ensure!(
            array.header.dtype.numeric(),
            "Cannot build a histogram for this dtype"
        );
        let total = array.slice_len(prefix);
        let step = if exact {
            1
        } else {
            total.div_ceil(250_000).max(1)
        };
        let mut range = [f64::INFINITY, f64::NEG_INFINITY];
        let mut count = 0u64;
        let mut mean = 0.;
        let mut m2 = 0.;
        let mut nonfinite = 0;
        for i in (0..total).step_by(step) {
            if i % (step * 8192) == 0 {
                ensure!(
                    generation.load(Ordering::Relaxed) == ticket,
                    "Operation cancelled"
                );
            }
            let x = array.slice_number(prefix, i, component).unwrap_or(f64::NAN);
            if !x.is_finite() {
                nonfinite += 1;
                continue;
            }
            range[0] = range[0].min(x);
            range[1] = range[1].max(x);
            count += 1;
            let delta = x - mean;
            mean += delta / count as f64;
            m2 += delta * (x - mean);
        }
        let data_range = range;
        let mut counts = vec![0u64; bins.clamp(8, 512)];
        if count > 0 {
            if range[0] == range[1] {
                let pad = (range[0].abs() * 0.01).max(0.5);
                range = [range[0] - pad, range[1] + pad];
            }
            for i in (0..total).step_by(step) {
                if i % (step * 8192) == 0 {
                    ensure!(
                        generation.load(Ordering::Relaxed) == ticket,
                        "Operation cancelled"
                    );
                }
                let x = array.slice_number(prefix, i, component).unwrap_or(f64::NAN);
                if x.is_finite() {
                    let index =
                        (((x - range[0]) / (range[1] - range[0])) * counts.len() as f64) as usize;
                    let index = index.min(counts.len() - 1);
                    counts[index] += 1;
                }
            }
        }
        Ok(Self {
            counts,
            range,
            data_range,
            count,
            sampled: total.div_ceil(step),
            total,
            nonfinite,
            mean,
            std: if count > 0 {
                (m2 / count as f64).sqrt()
            } else {
                f64::NAN
            },
        })
    }
    pub fn points(&self) -> Vec<[f64; 2]> {
        let width = (self.range[1] - self.range[0]) / self.counts.len() as f64;
        let mut points = Vec::with_capacity(self.counts.len() * 2);
        for (i, &count) in self.counts.iter().enumerate() {
            points.push([self.range[0] + i as f64 * width, count as f64]);
            points.push([self.range[0] + (i + 1) as f64 * width, count as f64]);
        }
        points
    }
}
