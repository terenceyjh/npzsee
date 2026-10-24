// SPDX-License-Identifier: GPL-3.0-or-later
use crate::{
    chart::Chart,
    i18n::{Language, UiText, visual_text},
};
use anyhow::{Context, Result};
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, RichText, Vec2};
use egui_extras::{Column, TableBuilder};
use npzsee::{
    analysis::{Envelope, Histogram, Layout, Series},
    data::{Array, Catalog, Component, size_label},
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

enum Work {
    Open(PathBuf),
    Load(Arc<Catalog>, usize),
    Plot(Series),
    Hist(Arc<Array>, Vec<usize>, Component, usize, bool),
}
enum Output {
    Catalog(Arc<Catalog>, f64),
    Array(Arc<Array>, f64, bool),
    Plot(Envelope, f64),
    Hist(Histogram, f64),
    Progress(u64, u64),
    Picked(Option<PathBuf>),
    Exported(UiText),
}
struct Event {
    ticket: u64,
    result: std::result::Result<Output, String>,
}
struct Worker {
    tx: mpsc::Sender<(u64, Work)>,
    rx: mpsc::Receiver<Event>,
    event_tx: mpsc::Sender<Event>,
    generation: Arc<AtomicU64>,
}
impl Worker {
    fn new(ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel::<(u64, Work)>();
        let (event_tx, event_rx) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let generation2 = generation.clone();
        let events = event_tx.clone();
        std::thread::spawn(move || {
            let mut cache: VecDeque<(usize, Arc<Array>)> = VecDeque::new();
            while let Ok((mut ticket, mut job)) = rx.recv() {
                while let Ok(new) = rx.try_recv() {
                    (ticket, job) = new;
                }
                if generation2.load(Ordering::Relaxed) != ticket {
                    continue;
                }
                let start = Instant::now();
                let result: Result<Output> = (|| {
                    Ok(match job {
                        Work::Open(path) => {
                            cache.clear();
                            Output::Catalog(
                                Arc::new(Catalog::open(&path)?),
                                start.elapsed().as_secs_f64(),
                            )
                        }
                        Work::Load(catalog, index) => {
                            if let Some(pos) = cache.iter().position(|(i, _)| *i == index) {
                                let item = cache.remove(pos).unwrap();
                                let array = item.1.clone();
                                cache.push_back(item);
                                Output::Array(array, start.elapsed().as_secs_f64(), true)
                            } else {
                                let mut last = Instant::now() - Duration::from_secs(1);
                                let array = Arc::new(Array::load(
                                    &catalog,
                                    index,
                                    &generation2,
                                    ticket,
                                    |done, total| {
                                        if last.elapsed() >= Duration::from_millis(80)
                                            || done == total
                                        {
                                            let _ = events.send(Event {
                                                ticket,
                                                result: Ok(Output::Progress(done, total)),
                                            });
                                            ctx.request_repaint();
                                            last = Instant::now();
                                        }
                                    },
                                )?);
                                cache.push_back((index, array.clone()));
                                while cache.len() > 2
                                    || (cache.len() > 1
                                        && cache
                                            .iter()
                                            .map(|(_, a)| a.header.nbytes as u64)
                                            .sum::<u64>()
                                            > 512 * 1024 * 1024)
                                {
                                    cache.pop_front();
                                }
                                Output::Array(array, start.elapsed().as_secs_f64(), false)
                            }
                        }
                        Work::Plot(series) => Output::Plot(
                            Envelope::build(series, &generation2, ticket)?,
                            start.elapsed().as_secs_f64(),
                        ),
                        Work::Hist(array, prefix, component, bins, exact) => Output::Hist(
                            Histogram::build(
                                &array,
                                &prefix,
                                component,
                                bins,
                                exact,
                                &generation2,
                                ticket,
                            )?,
                            start.elapsed().as_secs_f64(),
                        ),
                    })
                })();
                if generation2.load(Ordering::Relaxed) == ticket {
                    let _ = events.send(Event {
                        ticket,
                        result: result.map_err(|e| format!("{e:#}")),
                    });
                    ctx.request_repaint();
                }
            }
        });
        Self {
            tx,
            rx: event_rx,
            event_tx,
            generation,
        }
    }
    fn submit(&self, job: Work) {
        let ticket = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.tx.send((ticket, job));
    }
    fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Table,
    Plot,
    Histogram,
}

pub struct App {
    worker: Worker,
    catalog: Option<Arc<Catalog>>,
    array: Option<Arc<Array>>,
    selected: Option<usize>,
    filter: String,
    path_edit: String,
    tab: Tab,
    prefix: Vec<usize>,
    page: usize,
    column_page: usize,
    component: Component,
    layout: Layout,
    plot_row: usize,
    bins: usize,
    exact: bool,
    envelope: Option<Envelope>,
    histogram: Option<Histogram>,
    plot_chart: Chart,
    hist_chart: Chart,
    pending: bool,
    status: UiText,
    error: Option<String>,
    progress: Option<f32>,
    detail: Option<CellDetail>,
    language: Language,
    scalar_text: String,
    json: Option<serde_json::Value>,
    json_pretty: String,
    smoke: Option<Smoke>,
}
#[derive(Clone)]
struct CellDetail {
    location: String,
    text: String,
}

struct Smoke {
    out: PathBuf,
    step: usize,
    ready_since: Instant,
    shots: usize,
    started: Instant,
}
impl App {
    fn new(ctx: &egui::Context, path: Option<PathBuf>, smoke: Option<PathBuf>) -> Self {
        let language = Language::detect();
        let mut fonts = FontDefinitions::default();
        // System fonts stay local. The OFL fallback makes Chinese text portable.
        for path in [
            "C:/Windows/Fonts/msyh.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/System/Library/Fonts/PingFang.ttc",
        ] {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("cjk".into(), Arc::new(FontData::from_owned(data)));
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .push("cjk".into());
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .push("cjk".into());
                break;
            }
        }
        fonts.font_data.insert(
            "noto-cjk".into(),
            Arc::new(FontData::from_static(include_bytes!(
                "../../assets/NotoSansCJKsc-Regular.otf"
            ))),
        );
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("noto-cjk".into());
        }
        for (name, bytes) in [
            (
                "noto-sans",
                include_bytes!("../../assets/NotoSans-Regular.ttf").as_slice(),
            ),
            (
                "noto-arabic",
                include_bytes!("../../assets/NotoSansArabic-Regular.ttf").as_slice(),
            ),
        ] {
            fonts
                .font_data
                .insert(name.into(), Arc::new(FontData::from_static(bytes)));
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .insert(0, name.into());
            }
        }
        ctx.set_fonts(fonts);
        ctx.set_visuals(egui::Visuals::light());
        ctx.global_style_mut(|style| {
            style.spacing.item_spacing = Vec2::new(10., 8.);
            style.spacing.button_padding = Vec2::new(10., 6.);
        });
        let mut app = Self {
            worker: Worker::new(ctx.clone()),
            catalog: None,
            array: None,
            selected: None,
            filter: String::new(),
            path_edit: String::new(),
            tab: Tab::Table,
            prefix: Vec::new(),
            page: 0,
            column_page: 0,
            component: Component::Real,
            layout: Layout::Row,
            plot_row: 0,
            bins: 128,
            exact: false,
            envelope: None,
            histogram: None,
            plot_chart: Chart::default(),
            hist_chart: Chart {
                fill_to_zero: true,
                ..Default::default()
            },
            pending: false,
            status: language.text("welcome"),
            language,
            error: None,
            progress: None,
            detail: None,
            scalar_text: String::new(),
            json: None,
            json_pretty: String::new(),
            smoke: smoke.map(|out| Smoke {
                out,
                step: 0,
                ready_since: Instant::now(),
                shots: 0,
                started: Instant::now(),
            }),
        };
        if let Some(path) = path {
            app.open(path);
        }
        app
    }
    fn open(&mut self, path: PathBuf) {
        self.detail = None;
        self.filter.clear();
        self.worker.submit(Work::Open(path.clone()));
        self.path_edit = path.display().to_string();
        self.catalog = None;
        self.array = None;
        self.selected = None;
        self.pending = true;
        self.progress = None;
        self.status = self.language.text("opening");
        self.error = None;
        self.envelope = None;
        self.histogram = None;
        self.json = None;
        self.prefix.clear();
        self.tab = Tab::Table;
    }
    fn choose(&mut self, index: usize) {
        if let Some(catalog) = &self.catalog {
            self.worker.submit(Work::Load(catalog.clone(), index));
            self.detail = None;
            self.selected = Some(index);
            self.array = None;
            self.prefix.clear();
            self.page = 0;
            self.column_page = 0;
            self.tab = Tab::Table;
            self.pending = true;
            self.progress = None;
            self.error = None;
            self.envelope = None;
            self.histogram = None;
            self.json = None;
            self.scalar_text.clear();
            self.layout = Layout::Row;
            self.plot_row = 0;
            self.plot_chart.reset();
            self.hist_chart.reset();
            self.status = self
                .language
                .format("loading", &[("name", catalog.entries[index].name.clone())]);
        }
    }
    fn receive(&mut self) {
        while let Ok(event) = self.worker.rx.try_recv() {
            // Ticket 0 belongs to independent native file-dialog/export operations.
            if event.ticket != 0 && event.ticket != self.worker.generation.load(Ordering::Relaxed) {
                continue;
            }
            match event.result {
                Err(error) => {
                    self.pending = false;
                    self.progress = None;
                    self.error = Some(error);
                    self.status = self.language.text("operation_failed");
                }
                Ok(Output::Catalog(catalog, seconds)) => {
                    self.status = self.language.format(
                        "catalog_ready",
                        &[
                            ("count", catalog.entries.len().to_string()),
                            ("ms", format!("{:.2}", seconds * 1000.)),
                        ],
                    );
                    self.catalog = Some(catalog);
                    self.pending = false;
                    // Render the directory first; do not start decompressing a potentially huge first member.
                }
                Ok(Output::Array(array, seconds, cached)) => {
                    self.status = format!(
                        "{} · {} · {:.1} ms{}",
                        array.name,
                        size_label(array.header.nbytes as u64),
                        seconds * 1000.,
                        if cached {
                            self.language.text("cached")
                        } else {
                            UiText::default()
                        }
                    )
                    .into();
                    self.component = if array.header.dtype.kind == 'c' {
                        Component::Magnitude
                    } else {
                        Component::Real
                    };
                    self.array = Some(array);
                    self.pending = false;
                    self.progress = None;
                    self.prepare_text();
                }
                Ok(Output::Plot(envelope, seconds)) => {
                    self.status = self.language.format(
                        "plot_ready",
                        &[
                            ("count", envelope.series.len.to_string()),
                            ("ms", format!("{:.1}", seconds * 1000.)),
                        ],
                    );
                    self.envelope = Some(envelope);
                    self.pending = false;
                }
                Ok(Output::Hist(hist, seconds)) => {
                    self.status = self
                        .language
                        .format("hist_ready", &[("ms", format!("{:.1}", seconds * 1000.))]);
                    self.histogram = Some(hist);
                    self.pending = false;
                }
                Ok(Output::Progress(done, total)) => {
                    self.progress = Some(done as f32 / total.max(1) as f32);
                }
                Ok(Output::Picked(Some(path))) => self.open(path),
                Ok(Output::Picked(None)) => {}
                Ok(Output::Exported(message)) => self.status = message,
            }
        }
    }
    fn prepare_text(&mut self) {
        self.detail = None;
        self.scalar_text.clear();
        self.json = None;
        self.json_pretty.clear();
        if let Some(array) = &self.array
            && array.header.shape.len() == self.prefix.len()
        {
            self.scalar_text = array.display(&self.prefix);
            if array.header.dtype.text()
                && self.scalar_text.len() < 8 * 1024 * 1024
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(&self.scalar_text)
            {
                self.json_pretty = serde_json::to_string_pretty(&json).unwrap_or_default();
                self.json = Some(json);
            }
        }
    }
    fn navigate(&mut self, prefix: Vec<usize>) {
        self.worker.cancel();
        self.pending = false;
        self.prefix = prefix;
        self.page = 0;
        self.column_page = 0;
        self.plot_row = 0;
        self.layout = Layout::Row;
        self.envelope = None;
        self.histogram = None;
        self.plot_chart.reset();
        self.hist_chart.reset();
        self.tab = Tab::Table;
        self.prepare_text();
    }
    fn analyze(&mut self) {
        let Some(array) = &self.array else {
            return;
        };
        if !array.header.dtype.numeric() {
            return;
        }
        let job = match self.tab {
            Tab::Plot
                if self.envelope.is_none() && array.header.shape.len() - self.prefix.len() <= 2 =>
            {
                Some(Work::Plot(Series::new(
                    array.clone(),
                    self.prefix.clone(),
                    self.component,
                    self.layout,
                    self.plot_row,
                )))
            }
            Tab::Histogram if self.histogram.is_none() => Some(Work::Hist(
                array.clone(),
                self.prefix.clone(),
                self.component,
                self.bins,
                self.exact,
            )),
            _ => None,
        };
        if let Some(job) = job {
            self.worker.submit(job);
            self.pending = true;
            self.error = None;
            self.status = self.language.text("computing");
        }
    }
    fn dialog(&self, ctx: &egui::Context) {
        let tx = self.worker.event_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .add_filter("NumPy arrays", &["npy", "npz"])
                .pick_file();
            let _ = tx.send(Event {
                ticket: 0,
                result: Ok(Output::Picked(path)),
            });
            ctx.request_repaint();
        });
    }
    fn export(&self, ctx: &egui::Context) {
        let chart = if self.tab == Tab::Plot {
            &self.plot_chart
        } else {
            &self.hist_chart
        };
        if chart.last_points.is_empty() {
            return;
        }
        let title = format!(
            "{} {:?}",
            self.array
                .as_ref()
                .map(|a| a.name.as_str())
                .unwrap_or("npzsee"),
            self.prefix
        );
        let svg = chart.svg(&title);
        let language = self.language;
        let tx = self.worker.event_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("SVG figure", &["svg"])
                .set_file_name("npzsee-figure.svg")
                .save_file()
            {
                let result = std::fs::write(&path, svg)
                    .map(|_| {
                        Output::Exported(
                            language.format("exported", &[("path", path.display().to_string())]),
                        )
                    })
                    .map_err(|e| e.to_string());
                let _ = tx.send(Event { ticket: 0, result });
                ctx.request_repaint();
            }
        });
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("NPZSee").strong().size(23.));
            ui.separator();
            if ui.button(self.language.text("open")).clicked() {
                self.dialog(ui.ctx());
            }
            if ui
                .button(self.language.text(if ui.visuals().dark_mode {
                    "light"
                } else {
                    "dark"
                }))
                .clicked()
            {
                ui.ctx().set_visuals(if ui.visuals().dark_mode {
                    egui::Visuals::light()
                } else {
                    egui::Visuals::dark()
                });
            }
            ui.separator();
            ui.label(self.language.text("language"));
            let previous = self.language;
            egui::ComboBox::from_id_salt("language")
                .selected_text(self.language.name())
                .width(125.)
                .show_ui(ui, |ui| {
                    for language in Language::ALL {
                        ui.selectable_value(&mut self.language, language, language.name());
                    }
                });
            if self.language != previous {
                self.status = self.language.text("ready");
                if let Err(error) = self.language.save() {
                    self.error = Some(error.to_string());
                }
            }
        });
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.path_edit)
                    .hint_text(self.language.text("path_hint"))
                    .desired_width((ui.available_width() - 150.).max(100.)),
            );
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.open(PathBuf::from(&self.path_edit));
            }
            if ui
                .add_enabled(
                    self.catalog.is_some(),
                    egui::Button::new(self.language.text("reload")),
                )
                .clicked()
                && let Some(catalog) = &self.catalog
            {
                self.open(catalog.path.clone());
            }
        });
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.);
        ui.label(self.language.text("arrays").heading());
        let Some(catalog) = self.catalog.clone() else {
            ui.label(self.language.text("sidebar_empty"));
            return;
        };
        ui.label(
            RichText::new(
                catalog
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            )
            .small(),
        );
        ui.label(self.language.format(
            "array_count",
            &[("count", catalog.entries.len().to_string())],
        ));
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .hint_text(self.language.text("filter"))
                .desired_width(f32::INFINITY),
        );
        ui.separator();
        let filter = self.filter.to_lowercase();
        let indices: Vec<_> = catalog
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.name.to_lowercase().contains(&filter))
            .map(|(i, _)| i)
            .collect();
        let mut choose = None;
        egui::ScrollArea::vertical().id_salt("arrays").show_rows(
            ui,
            54.,
            indices.len(),
            |ui, visible| {
                for row in visible {
                    let i = indices[row];
                    let e = &catalog.entries[i];
                    let label = format!("{}\n{}", e.name, size_label(e.bytes));
                    let response = ui.add_sized(
                        [ui.available_width(), 50.],
                        egui::Button::new(label).selected(self.selected == Some(i)),
                    );
                    if response.clicked() {
                        choose = Some(i);
                    }
                    response.on_hover_text(self.language.format(
                        "entry_sizes",
                        &[
                            ("original", size_label(e.bytes)),
                            ("compressed", size_label(e.compressed_bytes)),
                        ],
                    ));
                }
            },
        );
        if let Some(index) = choose {
            self.choose(index);
        }
    }
    fn body(&mut self, ui: &mut egui::Ui) {
        if let Some(error) = self.error.clone() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.colored_label(Color32::from_rgb(190, 65, 55), error);
            });
        }
        let Some(array) = self.array.clone() else {
            ui.add_space(70.);
            ui.vertical_centered(|ui| {
                ui.label(
                    (if self.pending {
                        self.language.text("preparing")
                    } else if self.catalog.is_some() {
                        self.language.text("select_array")
                    } else {
                        self.language.text("headline")
                    })
                    .heading(),
                );
                ui.add_space(12.);
                ui.label(format!(
                    "{} · {} · {}",
                    self.language.text("table"),
                    self.language.text("plot"),
                    self.language.text("histogram")
                ));
                ui.label(self.language.text("intro"));
                if let Some(progress) = self.progress {
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .show_percentage()
                            .desired_width(340.),
                    );
                }
                if self.pending {
                    ui.spinner();
                    if ui.button(self.language.text("cancel")).clicked() {
                        self.worker.cancel();
                        self.pending = false;
                        self.status = self.language.text("cancelled");
                    }
                }
            });
            return;
        };
        ui.heading(&array.name);
        ui.label(
            RichText::new(format!(
                "shape {:?}   ·   {}   ·   {}   ·   {} order",
                array.header.shape,
                array.header.dtype.label,
                size_label(array.header.nbytes as u64),
                if array.header.fortran { "Fortran" } else { "C" }
            ))
            .monospace()
            .color(Color32::from_gray(120)),
        );
        let mut go = None;
        ui.horizontal_wrapped(|ui| {
            if ui.button(self.language.text("root")).clicked() {
                go = Some(Vec::new());
            }
            for (n, &index) in self.prefix.iter().enumerate() {
                ui.label("/");
                if ui.button(format!("[{index}]")).clicked() {
                    go = Some(self.prefix[..=n].to_vec());
                }
            }
            if !self.prefix.is_empty() && ui.button(self.language.text("back")).clicked() {
                go = Some(self.prefix[..self.prefix.len() - 1].to_vec());
            }
        });
        if let Some(prefix) = go {
            self.navigate(prefix);
        }
        ui.separator();
        let mut tab = self.tab;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut tab, Tab::Table, self.language.text("table"));
            ui.selectable_value(&mut tab, Tab::Plot, self.language.text("plot"));
            ui.selectable_value(&mut tab, Tab::Histogram, self.language.text("histogram"));
            if self.pending {
                ui.spinner();
            }
            if array.header.dtype.kind == 'c' {
                let old = self.component;
                egui::ComboBox::from_id_salt("component")
                    .selected_text(self.language.text(match self.component {
                        Component::Real => "real",
                        Component::Imag => "imag",
                        Component::Magnitude => "magnitude",
                        Component::Phase => "phase",
                    }))
                    .show_ui(ui, |ui| {
                        for (c, label) in [
                            (Component::Real, self.language.text("real")),
                            (Component::Imag, self.language.text("imag")),
                            (Component::Magnitude, self.language.text("magnitude")),
                            (Component::Phase, self.language.text("phase")),
                        ] {
                            ui.selectable_value(&mut self.component, c, label);
                        }
                    });
                if old != self.component {
                    self.envelope = None;
                    self.histogram = None;
                    self.plot_chart.reset();
                    self.hist_chart.reset();
                    self.analyze();
                }
            }
            if self.tab != Tab::Table
                && ui
                    .add_enabled(
                        !self.pending,
                        egui::Button::new(self.language.text("export")),
                    )
                    .clicked()
            {
                self.export(ui.ctx());
            }
        });
        if tab != self.tab {
            self.worker.cancel();
            self.pending = false;
            self.tab = tab;
            self.analyze();
        }
        ui.separator();
        match self.tab {
            Tab::Table => self.table_view(ui, &array),
            Tab::Plot => self.plot(ui, &array),
            Tab::Histogram => self.hist(ui, &array),
        }
    }
    fn table_view(&mut self, ui: &mut egui::Ui, array: &Array) {
        if let Some(detail) = self.detail.clone() {
            let mut close = false;
            egui::Panel::left("cell-details")
                .default_size(340.)
                .size_range(220.0..=600.0)
                .resizable(true)
                .show(ui, |ui| {
                    ui.label(self.language.text("detail").heading());
                    ui.label(&detail.location);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(self.language.text("copy")).clicked() {
                            ui.ctx().copy_text(detail.text.clone());
                        }
                        close = ui.button(self.language.text("close_detail")).clicked();
                    });
                    ui.separator();
                    egui::ScrollArea::both()
                        .id_salt("detail-scroll")
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(visual_text(&detail.text))
                                    .selectable(true)
                                    .wrap(),
                            );
                        });
                });
            if close {
                self.detail = None;
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| self.table(ui, array));
        } else {
            self.table(ui, array);
        }
    }
    fn table(&mut self, ui: &mut egui::Ui, array: &Array) {
        let shape = &array.header.shape[self.prefix.len()..];
        if shape.is_empty() {
            if let Some(json) = &self.json {
                ui.horizontal(|ui| {
                    ui.label(self.language.text("json"));
                    if ui.button(self.language.text("copy_json")).clicked() {
                        ui.ctx().copy_text(self.json_pretty.clone());
                    }
                });
                egui::ScrollArea::both().id_salt("json").show(ui, |ui| {
                    json_tree(ui, &array.name, json, 0, self.language, &mut self.detail);
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(self.language.text("scalar"));
                    if ui.button(self.language.text("copy")).clicked() {
                        ui.ctx().copy_text(self.scalar_text.clone());
                    }
                });
                egui::ScrollArea::both().id_salt("scalar").show(ui, |ui| {
                    ui.add(
                        egui::Label::new(visual_text(&self.scalar_text))
                            .selectable(true)
                            .wrap(),
                    );
                });
            }
            return;
        }
        let rows = shape[0];
        if rows == 0 || shape.contains(&0) {
            ui.label(
                self.language
                    .format("empty", &[("shape", format!("{shape:?}"))]),
            );
            return;
        }
        let nested = shape.len() > 2;
        let matrix = shape.len() == 2;
        let columns = if matrix { shape[1] } else { 1 };
        const PAGE: usize = 10_000;
        let pages = rows.div_ceil(PAGE);
        self.page = self.page.min(pages.saturating_sub(1));
        ui.horizontal_wrapped(|ui| {
            ui.label(self.language.format(
                "slice",
                &[("shape", format!("{shape:?}")), ("rows", rows.to_string())],
            ));
            if nested {
                ui.label(self.language.text("enter_subarray"));
            } else if matrix {
                ui.label(self.language.text("enter_row"));
            }
            if pages > 1 {
                if ui.button(self.language.text("previous")).clicked() {
                    self.page = self.page.saturating_sub(1);
                }
                if ui.button(self.language.text("next")).clicked() {
                    self.page = (self.page + 1).min(pages - 1);
                }
                let mut page = self.page + 1;
                if ui
                    .add(
                        egui::DragValue::new(&mut page)
                            .range(1..=pages)
                            .prefix(self.language.text("page")),
                    )
                    .changed()
                {
                    self.page = page - 1;
                }
                ui.label(format!("/ {pages}"));
            }
            if matrix && columns > 16 {
                let max = columns.div_ceil(16);
                let mut col_page = self.column_page + 1;
                if ui
                    .add(
                        egui::DragValue::new(&mut col_page)
                            .range(1..=max)
                            .prefix(self.language.text("column_page")),
                    )
                    .changed()
                {
                    self.column_page = col_page - 1;
                }
                ui.label(
                    self.language
                        .format("all_columns", &[("pages", max.to_string())]),
                );
            }
        });
        let first = self.page * PAGE;
        let count = (rows - first).min(PAGE);
        let col_start = if matrix { self.column_page * 16 } else { 0 };
        let col_end = if matrix {
            (col_start + 16).min(columns)
        } else {
            1
        };
        ui.label(RichText::new(self.language.text("click_detail")).small());
        let mut enter = None;
        let mut detail = None;
        egui::ScrollArea::horizontal()
            .id_salt(("table-x", self.selected, &self.prefix))
            .show(ui, |ui| {
                let mut table = TableBuilder::new(ui)
                    .id_salt((
                        "values",
                        self.selected,
                        &self.prefix,
                        self.page,
                        self.column_page,
                    ))
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(85.).at_least(70.));
                for _ in col_start..col_end {
                    table = table.column(if matrix {
                        Column::initial(150.).at_least(90.)
                    } else {
                        Column::remainder().at_least(300.)
                    });
                }
                table
                    .header(28., |mut header| {
                        header.col(|ui| {
                            ui.strong(self.language.text("index"));
                        });
                        for col in col_start..col_end {
                            header.col(|ui| {
                                ui.strong(if nested {
                                    self.language.text("subarray")
                                } else if matrix {
                                    format!("[{col}]").into()
                                } else {
                                    self.language.text("value")
                                });
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(27., count, |mut row| {
                            let index = first + row.index();
                            row.col(|ui| {
                                if matrix || nested {
                                    if ui.button(format!("[{index}] ›")).clicked() {
                                        enter = Some(index);
                                    }
                                } else {
                                    ui.monospace(index.to_string());
                                }
                            });
                            for col in col_start..col_end {
                                row.col(|ui| {
                                    let mut indices = self.prefix.clone();
                                    indices.push(index);
                                    if nested {
                                        if ui
                                            .button(self.language.format(
                                                "open_shape",
                                                &[("shape", format!("{:?}", &shape[1..]))],
                                            ))
                                            .clicked()
                                        {
                                            enter = Some(index);
                                        }
                                    } else {
                                        if matrix {
                                            indices.push(col);
                                        }
                                        let value = array.display(&indices);
                                        let preview: String = value
                                            .chars()
                                            .take(180)
                                            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                                            .collect();
                                        let response = ui.add(
                                            egui::Label::new(visual_text(&preview))
                                                .truncate()
                                                .sense(egui::Sense::click()),
                                        );
                                        if response.clicked() {
                                            detail = Some(CellDetail {
                                                location: format!("{} {indices:?}", array.name),
                                                text: value,
                                            });
                                        }
                                    }
                                });
                            }
                        });
                    });
            });
        if let Some(index) = enter {
            let mut prefix = self.prefix.clone();
            prefix.push(index);
            self.navigate(prefix);
        }
        if let Some(value) = detail {
            self.detail = Some(value);
        }
    }
    fn plot(&mut self, ui: &mut egui::Ui, array: &Arc<Array>) {
        if !array.header.dtype.numeric() {
            ui.label(self.language.text("non_numeric_plot"));
            return;
        }
        let shape = &array.header.shape[self.prefix.len()..];
        if shape.len() > 2 {
            ui.label(self.language.text("choose_slice"));
            return;
        }
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            if let [rows, cols] = shape {
                let old = self.layout;
                egui::ComboBox::from_id_salt("plot-layout")
                    .selected_text(match self.layout {
                        Layout::Row => self.language.text("by_row"),
                        Layout::XyRows => self.language.text("xy_rows"),
                        Layout::XyColumns => self.language.text("xy_columns"),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.layout,
                            Layout::Row,
                            self.language.text("by_row"),
                        );
                        if *rows == 2 {
                            ui.selectable_value(
                                &mut self.layout,
                                Layout::XyRows,
                                self.language.text("xy_rows"),
                            );
                        }
                        if *cols == 2 {
                            ui.selectable_value(
                                &mut self.layout,
                                Layout::XyColumns,
                                self.language.text("xy_columns"),
                            );
                        }
                    });
                changed |= old != self.layout;
                if self.layout == Layout::Row {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut self.plot_row)
                                .range(0..=rows.saturating_sub(1))
                                .prefix(self.language.text("row")),
                        )
                        .changed();
                }
            }
            if ui.button(self.language.text("reset")).clicked() {
                self.plot_chart.reset();
            }
            if ui
                .checkbox(&mut self.plot_chart.log_x, self.language.text("log_x"))
                .changed()
            {
                self.plot_chart.reset();
            }
            if ui
                .checkbox(&mut self.plot_chart.log_y, self.language.text("log_y"))
                .changed()
            {
                self.plot_chart.reset();
            }
            ui.label(self.language.text("plot_controls"));
        });
        if changed {
            self.envelope = None;
            self.plot_chart.reset();
            self.analyze();
        }
        let Some(envelope) = &self.envelope else {
            ui.label(if self.pending {
                self.language.text("indexing")
            } else {
                self.language.text("no_plot")
            });
            return;
        };
        if envelope.finite == 0 {
            ui.label(self.language.text("no_finite"));
            return;
        }
        if self.layout != Layout::Row {
            ui.label(self.language.text("xy_hint"));
        }
        if let Some(point) = self.plot_chart.last_hover
            && self.layout == Layout::Row
        {
            let index =
                (point[0].round().max(0.) as usize).min(envelope.series.len.saturating_sub(1));
            ui.label(self.language.format(
                "sample",
                &[
                    ("index", index.to_string()),
                    ("value", envelope.series.point(index)[1].to_string()),
                ],
            ));
        } else {
            ui.label(
                self.language
                    .format("samples", &[("count", envelope.series.len.to_string())]),
            );
        }
        self.plot_chart.show(
            ui,
            envelope.x_range,
            envelope.y_range,
            &if self.layout == Layout::Row {
                self.language.text("sample_index")
            } else {
                "X".into()
            },
            &self.language.text("value"),
            |range, width| envelope.points(range, width),
        );
    }
    fn hist(&mut self, ui: &mut egui::Ui, array: &Arc<Array>) {
        if !array.header.dtype.numeric() {
            ui.label(self.language.text("non_numeric_hist"));
            return;
        }
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(self.language.text("bins"));
            changed |= ui
                .add(egui::DragValue::new(&mut self.bins).range(8..=512))
                .changed();
            changed |= ui
                .checkbox(&mut self.exact, self.language.text("exact_all"))
                .changed();
            if ui
                .checkbox(&mut self.hist_chart.log_y, self.language.text("log_count"))
                .changed()
            {
                self.hist_chart.reset();
            }
            if ui.button(self.language.text("reset")).clicked() {
                self.hist_chart.reset();
            }
        });
        if changed {
            self.histogram = None;
            self.hist_chart.reset();
            self.analyze();
        }
        let Some(hist) = &self.histogram else {
            ui.label(self.language.text("hist_computing"));
            return;
        };
        if hist.count == 0 {
            ui.label(self.language.text("no_finite"));
            return;
        }
        ui.label(self.language.format(
            "hist_summary",
            &[
                (
                    "mode",
                    self.language.logical_text(if hist.sampled == hist.total {
                        "exact"
                    } else {
                        "approximate"
                    }),
                ),
                ("sampled", hist.sampled.to_string()),
                ("total", hist.total.to_string()),
                ("finite", hist.count.to_string()),
                ("nonfinite", hist.nonfinite.to_string()),
            ],
        ));
        ui.label(self.language.format(
            "statistics",
            &[
                ("min", crate::chart::tick(hist.data_range[0])),
                ("max", crate::chart::tick(hist.data_range[1])),
                ("mean", crate::chart::tick(hist.mean)),
                ("std", crate::chart::tick(hist.std)),
            ],
        ));
        let max = *hist.counts.iter().max().unwrap_or(&1) as f64;
        self.hist_chart.show(
            ui,
            hist.range,
            [0., max],
            &self.language.text("value"),
            &self.language.text("count"),
            |_, _| hist.points(),
        );
    }
    fn smoke_tick(&mut self, ctx: &egui::Context) {
        let Some(smoke) = &mut self.smoke else {
            return;
        };
        ctx.request_repaint_after(Duration::from_millis(40));
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let path = smoke.out.join(format!("ui-{}.png", smoke.shots));
                let _ = std::fs::create_dir_all(&smoke.out);
                let _ = image::save_buffer(
                    path,
                    &bytes,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                );
                smoke.shots += 1;
            }
        }
        if smoke.started.elapsed() > Duration::from_secs(90) {
            let _ = std::fs::write(
                smoke.out.join("smoke-error.txt"),
                format!(
                    "timeout step={} status={} error={:?}",
                    smoke.step, self.status, self.error
                ),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.pending {
            smoke.ready_since = Instant::now();
            return;
        }
        if smoke.ready_since.elapsed() < Duration::from_millis(400) {
            return;
        }
        let step = smoke.step;
        smoke.step += 1;
        smoke.ready_since = Instant::now();
        let valid = match step {
            0 => self.catalog.is_some() && self.array.is_none(),
            2 => self.json.is_some(),
            4 => self.array.as_ref().is_some_and(|a| a.name == "voltage") && self.tab == Tab::Table,
            6 => self.envelope.as_ref().is_some_and(|e| e.finite > 0),
            8 => self.histogram.as_ref().is_some_and(|h| h.count > 0),
            12 => self.prefix == [1],
            14 => self.prefix == [1, 2],
            _ => true,
        };
        if !valid || self.error.is_some() {
            let smoke = self.smoke.as_ref().unwrap();
            let _ = std::fs::write(
                smoke.out.join("smoke-error.txt"),
                format!(
                    "failed step={step} status={} error={:?}",
                    self.status, self.error
                ),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        match step {
            0 => ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default())),
            1 => {
                if let Some(c) = &self.catalog
                    && let Some(i) = c.entries.iter().position(|e| e.name == "metadata_json")
                {
                    self.choose(i);
                }
            }
            2 => ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default())),
            3 => {
                if let Some(c) = &self.catalog
                    && let Some(i) = c.entries.iter().position(|e| e.name == "voltage")
                {
                    self.choose(i);
                }
            }
            4 => ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default())),
            5 => {
                self.tab = Tab::Plot;
                self.analyze();
            }
            6 => ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default())),
            7 => {
                self.tab = Tab::Histogram;
                self.analyze();
            }
            8 => ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default())),
            9 => {
                if let Some(catalog) = &self.catalog
                    && let Some(i) = catalog.entries.iter().position(|e| e.name == "cube")
                {
                    self.choose(i);
                } else {
                    self.finish_smoke(ctx);
                }
            }
            10 | 12 | 14 | 16 => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()))
            }
            11 => self.navigate(vec![1]),
            13 => self.navigate(vec![1, 2]),
            15 => {
                self.tab = Tab::Plot;
                self.analyze();
            }
            17 => {
                if let Some(catalog) = &self.catalog
                    && let Some(index) = catalog
                        .entries
                        .iter()
                        .position(|entry| entry.name == "long_text")
                {
                    self.choose(index);
                } else {
                    self.finish_smoke(ctx);
                }
            }
            18 | 20 => ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default())),
            19 => {
                if let Some(array) = &self.array {
                    self.detail = Some(CellDetail {
                        location: "long_text [0]".into(),
                        text: array.display(&[0]),
                    });
                }
            }
            21 => self.finish_smoke(ctx),
            _ => {}
        }
    }
    fn finish_smoke(&self, ctx: &egui::Context) {
        let smoke = self.smoke.as_ref().unwrap();
        let report = format!(
            "screenshots={}\narrays={}\nstatus={}\nerror={:?}\n",
            smoke.shots,
            self.catalog.as_ref().map_or(0, |c| c.entries.len()),
            self.status,
            self.error
        );
        let _ = std::fs::write(smoke.out.join("smoke.txt"), report);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

fn json_tree(
    ui: &mut egui::Ui,
    name: &str,
    value: &serde_json::Value,
    depth: usize,
    language: Language,
    detail: &mut Option<CellDetail>,
) {
    if depth > 48 {
        ui.label(language.text("json_depth"));
        return;
    }
    match value {
        serde_json::Value::Object(values) => {
            egui::CollapsingHeader::new(language.format(
                "json_items",
                &[("name", name.into()), ("count", values.len().to_string())],
            ))
            .id_salt((name, depth))
            .default_open(depth < 2)
            .show(ui, |ui| {
                for (key, value) in values {
                    json_tree(ui, key, value, depth + 1, language, detail);
                }
            });
        }
        serde_json::Value::Array(values) => {
            egui::CollapsingHeader::new(language.format(
                "json_items",
                &[("name", name.into()), ("count", values.len().to_string())],
            ))
            .id_salt((name, depth))
            .default_open(depth < 1)
            .show(ui, |ui| {
                // Paginate large JSON arrays without instantiating millions of widgets.
                let id = ui.id().with((name, "json-page"));
                let mut page = ui.data_mut(|d| d.get_temp::<usize>(id).unwrap_or(0));
                let pages = values.len().div_ceil(100).max(1);
                if pages > 1 {
                    ui.horizontal(|ui| {
                        ui.label(language.text("page"));
                        let mut number = page + 1;
                        ui.add(egui::DragValue::new(&mut number).range(1..=pages));
                        page = number - 1;
                        ui.label(format!("/ {pages}"));
                    });
                }
                ui.data_mut(|d| d.insert_temp(id, page));
                for (i, value) in values.iter().enumerate().skip(page * 100).take(100) {
                    json_tree(ui, &format!("[{i}]"), value, depth + 1, language, detail);
                }
            });
        }
        _ => {
            let text = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            let preview: String = text
                .chars()
                .take(160)
                .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                .collect();
            if ui
                .add(
                    egui::Label::new(visual_text(&format!("{name}: {preview}")))
                        .truncate()
                        .sense(egui::Sense::click()),
                )
                .clicked()
            {
                *detail = Some(CellDetail {
                    location: name.into(),
                    text,
                });
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.receive();
        self.plot_chart.language = self.language;
        self.hist_chart.language = self.language;
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.dialog(&ctx);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F5))
            && let Some(c) = &self.catalog
        {
            self.open(c.path.clone());
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S))
            && self.tab != Tab::Table
        {
            self.export(&ctx);
        }
        if let Some(path) =
            ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.open(path);
        }
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.pending {
                    ui.spinner();
                }
                ui.label(&self.status);
            });
        });
        egui::Panel::left("browser")
            .default_size(280.)
            .size_range(180.0..=520.)
            .resizable(true)
            .show(ui, |ui| self.sidebar(ui));
        egui::CentralPanel::default().show(ui, |ui| self.body(ui));
        self.smoke_tick(&ctx);
    }
}

pub fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let mut path = None;
    let mut smoke = None;
    let mut language = None;
    while let Some(arg) = args.next() {
        if arg == "--ui-smoke" {
            smoke = Some(PathBuf::from(
                args.next()
                    .context("--ui-smoke requires output directory")?,
            ));
        } else if arg == "--language" {
            let code = args
                .next()
                .context("--language requires en, zh-CN, fr, es, ru or ar")?;
            language = Some(Language::parse(&code.to_string_lossy()));
        } else if arg == "--version" || arg == "-V" {
            println!("npzsee {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        } else if arg == "--license" {
            println!(
                "{}\n{}",
                include_str!("../../NOTICE"),
                include_str!("../../LICENSE")
            );
            return Ok(());
        } else if arg == "--help" || arg == "-h" {
            println!(
                "NPZSee — multidimensional NumPy array viewer\nnpzsee [file.npy|file.npz] [--language en|zh-CN|fr|es|ru|ar] [--ui-smoke output-directory]"
            );
            return Ok(());
        } else {
            path = Some(PathBuf::from(arg));
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NPZSee · NumPy array viewer")
            .with_inner_size([1380., 820.])
            .with_min_inner_size([850., 560.]),
        ..Default::default()
    };
    eframe::run_native(
        "npzsee",
        options,
        Box::new(move |cc| {
            let mut app = App::new(&cc.egui_ctx, path, smoke);
            if let Some(language) = language {
                app.language = language;
                app.status = language.text("welcome");
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn array(shape: &[usize], text: bool) -> Arc<Array> {
        array_content(shape, text.then_some("测试 metadata"))
    }
    fn array_content(shape: &[usize], text: Option<&str>) -> Arc<Array> {
        let data: Vec<u8> = if let Some(text) = text {
            text.chars()
                .flat_map(|c| (c as u32).to_le_bytes())
                .collect()
        } else {
            (0..shape.iter().product::<usize>())
                .flat_map(|v| (v as f64).to_le_bytes())
                .collect()
        };
        let dtype = if text.is_some() {
            format!("<U{}", data.len() / 4)
        } else {
            "<f8".into()
        };
        let header = format!(
            "{{'descr': '{dtype}', 'fortran_order': False, 'shape': ({}), }}\n",
            shape.iter().map(|n| format!("{n},")).collect::<String>()
        );
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"\x93NUMPY\x01\x00").unwrap();
        file.write_all(&(header.len() as u16).to_le_bytes())
            .unwrap();
        file.write_all(header.as_bytes()).unwrap();
        file.write_all(&data).unwrap();
        let catalog = Catalog::open(file.path()).unwrap();
        Arc::new(Array::load(&catalog, 0, &AtomicU64::new(1), 1, |_, _| {}).unwrap())
    }
    fn labels(shape: &egui::Shape, output: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            egui::Shape::Text(text) => {
                output.push((
                    text.galley.text().to_string(),
                    text.pos + text.galley.size() * 0.5,
                ));
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    labels(shape, output);
                }
            }
            _ => {}
        }
    }
    fn frame(
        app: &mut App,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> Vec<(String, egui::Pos2)> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1100., 700.),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| app.body(ui));
        });
        output.textures_delta.clear(); // No GPU backend in these interaction tests.
        let mut result = Vec::new();
        for shape in output.shapes {
            labels(&shape.shape, &mut result);
        }
        result
    }
    fn click(app: &mut App, ctx: &egui::Context, label: &str) {
        let labels = frame(app, ctx, vec![]);
        let pos = labels
            .iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("missing widget {label}: {labels:?}"))
            .1;
        frame(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        frame(
            app,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
    }
    #[test]
    fn multi_dimension_clicks_and_breadcrumbs() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None, None);
        app.language = Language::Chinese;
        app.array = Some(array(&[2, 3, 4], false));
        click(&mut app, &ctx, "[0] ›");
        assert_eq!(app.prefix, [0]);
        click(&mut app, &ctx, "[1] ›");
        assert_eq!(app.prefix, [0, 1]);
        let visible = frame(&mut app, &ctx, vec![]);
        assert!(visible.iter().any(|(t, _)| t == "4"));
        click(&mut app, &ctx, "根数组");
        assert!(app.prefix.is_empty());
        assert!(app.tab == Tab::Table);
    }
    #[test]
    fn strings_render_and_both_charts_explain_incompatibility() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None, None);
        app.language = Language::Chinese;
        app.array = Some(array(&[], true));
        app.prepare_text();
        assert!(
            frame(&mut app, &ctx, vec![])
                .iter()
                .any(|(t, _)| t == "测试 metadata")
        );
        click(&mut app, &ctx, "曲线");
        assert!(app.tab == Tab::Plot);
        assert!(!app.pending);
        assert!(
            frame(&mut app, &ctx, vec![])
                .iter()
                .any(|(t, _)| t.contains("不是数值数据"))
        );
        click(&mut app, &ctx, "直方图");
        assert!(
            frame(&mut app, &ctx, vec![])
                .iter()
                .any(|(t, _)| t.contains("无法绘制直方图"))
        );
    }
    #[test]
    fn old_worker_results_cannot_replace_the_current_selection() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None, None);
        app.language = Language::Chinese;
        app.worker.generation.store(5, Ordering::Relaxed);
        app.pending = true;
        app.worker
            .event_tx
            .send(Event {
                ticket: 4,
                result: Ok(Output::Array(array(&[2], false), 0., false)),
            })
            .unwrap();
        app.receive();
        assert!(app.array.is_none());
        assert!(app.pending);
        app.worker
            .event_tx
            .send(Event {
                ticket: 5,
                result: Ok(Output::Array(array(&[3], false), 0., false)),
            })
            .unwrap();
        app.receive();
        assert_eq!(app.array.as_ref().unwrap().header.shape, [3]);
        assert!(!app.pending);
    }
    #[test]
    fn virtual_table_can_reach_last_page_and_all_columns() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None, None);
        app.language = Language::Chinese;
        app.array = Some(array(&[10001, 20], false));
        app.page = 1;
        app.column_page = 1;
        let visible = frame(&mut app, &ctx, vec![]);
        assert!(visible.iter().any(|(t, _)| t == "[10000] ›"));
        assert!(visible.iter().any(|(t, _)| t == "200019"));
    }

    #[test]
    fn clicking_long_cells_opens_and_closes_the_table_inspector_without_truncating_data() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None, None);
        app.language = Language::English;
        let original = "Long content 中文\nsecond line — русский — العربية\n".repeat(400);
        app.array = Some(array_content(&[1], Some(&original)));
        let preview = frame(&mut app, &ctx, vec![])
            .into_iter()
            .find(|(text, _)| text.starts_with("Long content"))
            .unwrap()
            .0;
        assert!(app.detail.is_none());
        click(&mut app, &ctx, &preview);
        assert_eq!(app.detail.as_ref().unwrap().text, original);
        let visible = frame(&mut app, &ctx, vec![]);
        assert!(visible.iter().any(|(text, _)| text == "Close details"));
        click(&mut app, &ctx, "Close details");
        assert!(app.detail.is_none());
    }

    #[test]
    fn every_language_renders_array_navigation_and_toolbar() {
        for language in Language::ALL {
            let ctx = egui::Context::default();
            let mut app = App::new(&ctx, None, None);
            app.language = language;
            app.array = Some(array(&[2, 3, 4], false));
            let visible = frame(&mut app, &ctx, vec![]);
            assert!(visible.iter().any(|(text, _)| text == "[0] ›"));
            let mut output = ctx.run_ui(Default::default(), |ui| app.toolbar(ui));
            output.textures_delta.clear();
            assert!(!output.shapes.is_empty(), "{}", language.code());
        }
    }
}
