use std::fmt;

use tinyvec::TinyVec;
use tracing::field::{Field, Visit};
use tracing_subscriber::field::RecordFields;

use crate::util::Formatter;

pub trait RecordDisplay: RecordFields {
    fn find_map_display<T, F: Fn(&dyn fmt::Display) -> T>(&self, name: &str, f: F) -> Option<T>;
    fn record_display<F: FnMut(&Field, &dyn fmt::Display)>(&self, f: F);
}

#[derive(Debug)]
pub struct Data {
    // start: Instant,
    map: TinyVec<[(&'static str, String); 3]>,
}

impl Data {
    pub fn new<T: RecordFields>(attrs: T) -> Self {
        let mut data = Data {
            // start: Instant::now(),
            map: TinyVec::new(),
        };

        attrs.record(&mut data);
        data
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.map
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.as_str())
    }
}

impl std::ops::Index<&str> for Data {
    type Output = str;

    fn index(&self, index: &str) -> &Self::Output {
        self.get(index).unwrap_or("[internal error: missing key]")
    }
}

impl Visit for Data {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.map.push((field.name(), format!("{:?}", value)));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.map.push((field.name(), value.into()));
    }
}

impl<T: RecordFields> RecordDisplay for T {
    fn find_map_display<V, F: Fn(&dyn fmt::Display) -> V>(&self, name: &str, f: F) -> Option<V> {
        let mut value = None;
        self.record_display(|field, item| {
            if field.name() == name {
                value = Some(f(item));
            }
        });
        value
    }

    fn record_display<F: FnMut(&Field, &dyn fmt::Display)>(&self, f: F) {
        struct DisplayVisit<F>(F);

        impl<F: FnMut(&Field, &dyn fmt::Display)> Visit for DisplayVisit<F> {
            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                (self.0)(field, &Formatter(|f| value.fmt(f)));
            }

            fn record_str(&mut self, field: &Field, value: &str) {
                (self.0)(field, &value)
            }
        }

        self.record(&mut DisplayVisit(f));
    }
}
