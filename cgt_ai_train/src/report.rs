use burn::{
    data::dataloader::Progress,
    train::{
        Interrupter,
        metric::{
            CpuUse, Metric, MetricAttributes, MetricDefinition, MetricEntry, MetricId,
            MetricMetadata, Numeric, NumericAttributes, NumericEntry, SerializedEntry,
        },
        renderer::{
            MetricState, MetricsRenderer, MetricsRendererTraining, ProgressType, TrainingProgress,
            tui::TuiMetricsRendererWrapper,
        },
    },
};
use nvml_wrapper::Nvml;
use std::{
    io::IsTerminal,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

pub struct IterationReport {
    pub iteration: usize,
    pub buffer: usize,
    pub new_examples: usize,
    pub mean_length: f64,
    pub left_wins: f64,
    pub policy_loss: f32,
    pub value_loss: f32,
    pub steps: usize,
    pub self_play_time: f64,
    pub train_time: f64,
}

impl IterationReport {
    fn line(&self, done: usize, total: usize) -> String {
        format!(
            "iter {:5} (+{done}/{total}) | buffer={:7} new={:6} | len={:5.1} left_wins={:.2} | \
             policy_loss={:.4} value_loss={:.4} steps={} | self_play={:.1}s train={:.1}s",
            self.iteration,
            self.buffer,
            self.new_examples,
            self.mean_length,
            self.left_wins,
            self.policy_loss,
            self.value_loss,
            self.steps,
            self.self_play_time,
            self.train_time,
        )
    }
}

#[derive(Clone, Copy)]
pub enum Phase {
    SelfPlay { moves: usize, games: usize },
    Training { steps: usize, total: usize },
}

struct MetricSpec {
    name: &'static str,
    unit: Option<&'static str>,
    higher_is_better: bool,
    value: fn(&IterationReport) -> f64,
    precision: usize,
}

const METRICS: [MetricSpec; 6] = [
    MetricSpec {
        name: "Policy loss",
        unit: None,
        higher_is_better: false,
        value: |r| f64::from(r.policy_loss),
        precision: 4,
    },
    MetricSpec {
        name: "Value loss",
        unit: None,
        higher_is_better: false,
        value: |r| f64::from(r.value_loss),
        precision: 4,
    },
    MetricSpec {
        name: "Game length",
        unit: Some("plies"),
        higher_is_better: true,
        value: |r| r.mean_length,
        precision: 1,
    },
    MetricSpec {
        name: "Left wins",
        unit: Some("%"),
        higher_is_better: true,
        value: |r| 100.0 * r.left_wins,
        precision: 1,
    },
    MetricSpec {
        name: "Self-play time",
        unit: Some("s"),
        higher_is_better: false,
        value: |r| r.self_play_time,
        precision: 1,
    },
    MetricSpec {
        name: "Train time",
        unit: Some("s"),
        higher_is_better: false,
        value: |r| r.train_time,
        precision: 1,
    },
];

const USAGE_INTERVAL: Duration = Duration::from_secs(1);

/// Burn's TUI takes the progress of the run to be the iterations done plus the fraction of the top
/// bar, and works out its ETA as if every iteration had as many units of progress. So the top bar
/// follows the whole iteration in a fixed number of units, as a top bar that started again for
/// training would move the bottom bar back.
const ITERATION_UNITS: usize = 1000;

/// Part of an iteration taken by self-play until an iteration has trained, about what the presets
/// take
const SELF_PLAY_SHARE: f64 = 0.75;

struct Usage {
    cpu: CpuUse,
    cpu_id: MetricId,
    gpu: Option<(Nvml, MetricId, MetricId)>,
    last: Instant,
}

fn register(
    renderer: &mut TuiMetricsRendererWrapper,
    name: &str,
    attributes: MetricAttributes,
) -> MetricId {
    let id = MetricId::new(Arc::new(name.to_string()));
    renderer.register_metric(MetricDefinition {
        metric_id: id.clone(),
        name: name.to_string(),
        description: None,
        attributes,
    });
    id
}

fn numeric(unit: Option<&str>, higher_is_better: bool) -> MetricAttributes {
    NumericAttributes {
        unit: unit.map(String::from),
        higher_is_better,
    }
    .into()
}

struct Tui {
    renderer: TuiMetricsRendererWrapper,
    metrics: Vec<(MetricId, &'static MetricSpec)>,
    buffer_id: MetricId,
    usage: Usage,
    first_iteration: usize,
    last_iteration: usize,
    iteration: usize,
    mean_length: f64,
    /// Part of the last iteration that trained taken by self-play
    self_play_share: f64,
    checkpoint: Option<(usize, PathBuf)>,
}

impl Tui {
    fn push(&mut self, id: &MetricId, precision: usize, value: f64) {
        let formatted = format!("{value:.precision$}");
        self.renderer.update_train(MetricState::Numeric(
            MetricEntry::new(
                id.clone(),
                SerializedEntry::new(formatted, value.to_string()),
            ),
            NumericEntry::Value(value),
        ));
    }

    fn push_text(&mut self, id: &MetricId, text: String) {
        self.renderer
            .update_train(MetricState::Generic(MetricEntry::new(
                id.clone(),
                SerializedEntry::new(text.clone(), text),
            )));
    }

    fn sample_usage(&mut self, force: bool) {
        if !force && self.usage.last.elapsed() < USAGE_INTERVAL {
            return;
        }
        self.usage.last = Instant::now();
        let metadata = MetricMetadata {
            progress: Progress::new(0, 1),
            global_progress: Progress::new(0, 1),
            iteration: None,
            lr: None,
        };
        self.usage.cpu.update(&(), &metadata);
        let cpu = self.usage.cpu.value().current();
        let cpu_id = self.usage.cpu_id.clone();
        self.push(&cpu_id, 1, cpu);
        let gpu = self
            .usage
            .gpu
            .as_ref()
            .and_then(|(nvml, usage_id, memory_id)| {
                let device = nvml.device_by_index(0).ok()?;
                let usage = f64::from(device.utilization_rates().ok()?.gpu);
                let memory = device.memory_info().ok()?.used as f64 / 1e9;
                Some((usage_id.clone(), usage, memory_id.clone(), memory))
            });
        if let Some((usage_id, usage, memory_id, memory)) = gpu {
            self.push(&usage_id, 0, usage);
            self.push(&memory_id, 2, memory);
        }
    }
}

pub struct Reporter {
    tui: Option<Tui>,
    total: usize,
    done: usize,
    last_line: Option<String>,
}

impl Reporter {
    pub fn new(
        interrupter: &Interrupter,
        start_iteration: usize,
        iterations: usize,
        buffer: usize,
        typical_game_length: usize,
        plain: bool,
    ) -> Reporter {
        let tui = (!plain && std::io::stdout().is_terminal()).then(|| {
            let mut renderer =
                TuiMetricsRendererWrapper::new(interrupter.clone(), Some(start_iteration));
            let metrics = METRICS
                .iter()
                .map(|spec| {
                    let attributes = numeric(spec.unit, spec.higher_is_better);
                    (register(&mut renderer, spec.name, attributes), spec)
                })
                .collect();
            let buffer_id = register(&mut renderer, "Replay buffer", MetricAttributes::None);
            let usage = Usage {
                cpu: CpuUse::new(),
                cpu_id: register(&mut renderer, "CPU usage", numeric(Some("%"), false)),
                gpu: Nvml::init().ok().map(|nvml| {
                    (
                        nvml,
                        register(&mut renderer, "GPU usage", numeric(Some("%"), true)),
                        register(&mut renderer, "GPU memory", numeric(Some("GB"), false)),
                    )
                }),
                last: Instant::now(),
            };
            let mut tui = Tui {
                renderer,
                metrics,
                buffer_id,
                usage,
                first_iteration: start_iteration + 1,
                last_iteration: start_iteration + iterations,
                iteration: start_iteration + 1,
                mean_length: typical_game_length as f64,
                self_play_share: SELF_PLAY_SHARE,
                checkpoint: None,
            };
            // Burn 0.21's TUI divides by the number of plotted metrics when the
            // arrow keys switch plots, so one must have a value before the first
            // iteration finishes. Fixed upstream in 0.22.
            tui.sample_usage(true);
            let buffer_id = tui.buffer_id.clone();
            tui.push_text(&buffer_id, buffer.to_string());
            tui
        });
        Reporter {
            tui,
            total: iterations,
            done: 0,
            last_line: None,
        }
    }

    // Burn's progress panel has no labels, so the status lines name the bar
    // they describe. The top bar follows the current iteration, self-play and
    // then training, and the bottom bar the whole run.
    pub fn phase(&mut self, phase: Phase) {
        let Some(tui) = &mut self.tui else { return };
        tui.sample_usage(false);
        let share = tui.self_play_share;
        let (tag, progress, done) = match phase {
            Phase::SelfPlay { moves, games } => {
                let expected = (games as f64 * tui.mean_length).round() as usize;
                let progress = Progress::new(moves, expected.max(moves).max(1));
                let done = share * moves as f64 / progress.items_total as f64;
                ("Top bar: iteration, self-play moves", progress, done)
            }
            Phase::Training { steps, total } => {
                let progress = Progress::new(steps, total.max(1));
                let done = share + (1.0 - share) * steps as f64 / progress.items_total as f64;
                ("Top bar: iteration, training steps", progress, done)
            }
        };
        let mut status = vec![
            ProgressType::Detailed {
                tag: "Bottom bar: iterations".into(),
                progress: Progress::new(tui.iteration, tui.last_iteration),
            },
            ProgressType::Detailed {
                tag: tag.into(),
                progress,
            },
        ];
        if let Some((iteration, _)) = &tui.checkpoint {
            status.push(ProgressType::Value {
                tag: "Last checkpoint".into(),
                value: *iteration,
            });
        }
        tui.renderer.render_train(
            TrainingProgress {
                progress: Some(Progress::new(
                    (done * ITERATION_UNITS as f64) as usize,
                    ITERATION_UNITS,
                )),
                global_progress: Progress::new(tui.iteration, tui.last_iteration),
                iteration: Some(tui.iteration - tui.first_iteration),
            },
            status,
        );
    }

    pub fn iteration(&mut self, report: &IterationReport) {
        self.done += 1;
        let line = report.line(self.done, self.total);
        let Some(tui) = &mut self.tui else {
            println!("{line}");
            return;
        };
        self.last_line = Some(line);
        if report.mean_length.is_finite() {
            tui.mean_length = report.mean_length;
        }
        let time = report.self_play_time + report.train_time;
        if report.steps > 0 && time > 0.0 {
            tui.self_play_share = report.self_play_time / time;
        }
        tui.iteration = report.iteration + 1;
        for (id, spec) in tui.metrics.clone() {
            let value = (spec.value)(report);
            if !value.is_nan() {
                tui.push(&id, spec.precision, value);
            }
        }
        let buffer_id = tui.buffer_id.clone();
        tui.push_text(&buffer_id, report.buffer.to_string());
    }

    pub fn checkpoint(&mut self, iteration: usize, path: &Path) {
        match &mut self.tui {
            Some(tui) => tui.checkpoint = Some((iteration, path.to_path_buf())),
            None => println!("  saved checkpoint {}", path.display()),
        }
    }

    pub fn close(mut self) {
        let Some(tui) = self.tui.take() else { return };
        let checkpoint = tui.checkpoint.map(|(_, path)| path);
        drop(tui.renderer);
        if let Some(line) = self.last_line.take() {
            println!("{line}");
        }
        if let Some(path) = checkpoint {
            println!("  saved checkpoint {}", path.display());
        }
    }
}
