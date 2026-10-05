use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub label: String,
    pub percent: f64,
}

type ProgressCallback = dyn Fn(ProgressEvent) + Send + Sync;

#[derive(Clone)]
pub struct Progress {
    callback: Arc<ProgressCallback>,
    last: Arc<Mutex<Option<(Instant, String, f64)>>>,
    start: f64,
    span: f64,
}

impl Default for Progress {
    fn default() -> Self {
        Self::new(|_| {})
    }
}

impl Progress {
    pub fn new(callback: impl Fn(ProgressEvent) + Send + Sync + 'static) -> Self {
        Self {
            callback: Arc::new(callback),
            last: Arc::new(Mutex::new(None)),
            start: 0.,
            span: 1.,
        }
    }

    pub fn range(&self, start: f64, span: f64) -> Self {
        Self {
            start: self.start + start * self.span,
            span: self.span * span,
            ..self.clone()
        }
    }

    pub fn report(&self, label: &str, fraction: f64) {
        let percent = (self.start + fraction.clamp(0., 1.) * self.span) * 100.;
        let now = Instant::now();
        let mut last = self.last.lock().unwrap();
        if let Some((time, previous_label, previous_percent)) = &*last {
            if label == previous_label
                && percent < 100.
                && now.duration_since(*time) < Duration::from_millis(150)
            {
                return;
            }
            if label == previous_label && percent < *previous_percent {
                return;
            }
        }
        *last = Some((now, label.to_owned(), percent));
        drop(last);
        (self.callback)(ProgressEvent {
            label: label.to_owned(),
            percent,
        });
    }
}
