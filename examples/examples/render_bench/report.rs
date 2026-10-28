//! Frame and pass timings collected over a run, as a table and as JSON.

/// Milliseconds measured once per frame.
#[derive(Debug, Default)]
pub(crate) struct Samples(Vec<f64>);

impl Samples {
    pub(crate) fn push(&mut self, millis: f64) {
        self.0.push(millis);
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "frame counts are far below 2^52"
    )]
    pub(crate) fn mean(&self) -> f64 {
        if self.0.is_empty() {
            return 0.0;
        }
        self.0.iter().sum::<f64>() / self.0.len() as f64
    }

    /// The value `fraction` (0..=1) of the way up the sorted samples.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "an index into a short list"
    )]
    pub(crate) fn percentile(&self, fraction: f64) -> f64 {
        let mut sorted = self.0.clone();
        sorted.sort_by(f64::total_cmp);
        let last = sorted.len().saturating_sub(1);
        sorted
            .get((last as f64 * fraction).round() as usize)
            .copied()
            .unwrap_or(0.0)
    }
}

/// Everything one benchmark run measured.
#[derive(Debug)]
pub(crate) struct Report {
    pub(crate) scene: &'static str,
    pub(crate) adapter: String,
    pub(crate) size: (u32, u32),
    pub(crate) frames: usize,
    pub(crate) update: Samples,
    pub(crate) render: Samples,
    pub(crate) passes: Vec<(&'static str, Samples)>,
    pub(crate) gpu_timestamps: bool,
}

impl Report {
    /// Files one frame's GPU pass times under their labels.
    pub(crate) fn add_passes(&mut self, timings: &[meridian_sdk::PassTiming]) {
        for timing in timings {
            if let Some((_, samples)) = self
                .passes
                .iter_mut()
                .find(|(label, _)| *label == timing.label)
            {
                samples.push(timing.millis);
            } else {
                let mut samples = Samples::default();
                samples.push(timing.millis);
                self.passes.push((timing.label, samples));
            }
        }
    }

    /// The human-readable summary.
    pub(crate) fn table(&self) -> String {
        let mut out = format!(
            "scene {}  {}x{}  {} frames\nadapter: {}\n\n{:<40}{:>10}{:>10}{:>10}\n",
            self.scene,
            self.size.0,
            self.size.1,
            self.frames,
            self.adapter,
            "ms per frame",
            "mean",
            "p95",
            "max"
        );
        out += &row("CPU update (animation, scene)", &self.update);
        out += &row("CPU render (record + submit + present)", &self.render);
        if self.gpu_timestamps {
            out += &header(
                "
GPU pass",
            );
            for (label, samples) in &self.passes {
                out += &row(label, samples);
            }
        } else {
            out += "\nGPU pass times: this adapter has no timestamp queries\n";
        }
        out
    }

    /// The machine-readable summary.
    pub(crate) fn json(&self) -> String {
        let passes: Vec<String> = self
            .passes
            .iter()
            .map(|(label, samples)| format!("{}: {}", quote(label), stats(samples)))
            .collect();
        format!(
            "{{\"scene\": {}, \"adapter\": {}, \"width\": {}, \"height\": {}, \"frames\": {}, \"gpu_timestamps\": {}, \"cpu_update_ms\": {}, \"cpu_render_ms\": {}, \"gpu_passes_ms\": {{{}}}}}",
            quote(self.scene),
            quote(&self.adapter),
            self.size.0,
            self.size.1,
            self.frames,
            self.gpu_timestamps,
            stats(&self.update),
            stats(&self.render),
            passes.join(", ")
        )
    }
}

/// A column header line over [`row`]s.
fn header(title: &str) -> String {
    format!("{title:<40}{:>10}{:>10}{:>10}\n", "mean", "p95", "max")
}

fn row(label: &str, samples: &Samples) -> String {
    format!(
        "{label:<40}{:>10.3}{:>10.3}{:>10.3}\n",
        samples.mean(),
        samples.percentile(0.95),
        samples.percentile(1.0)
    )
}

fn stats(samples: &Samples) -> String {
    format!(
        "{{\"mean\": {:.4}, \"p95\": {:.4}, \"max\": {:.4}}}",
        samples.mean(),
        samples.percentile(0.95),
        samples.percentile(1.0)
    )
}

fn quote(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
