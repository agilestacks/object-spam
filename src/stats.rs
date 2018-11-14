
use std::fmt::{Formatter, Display, Result as FResult};
use serde_json;

const FIFTY : usize = 0;
const SEVENTY : usize = 1;
const NINETY : usize = 2;
const NINETY5 : usize = 3;
const NINETY9 : usize = 4;
type Histogram = [f64; 5];
type HistCounts = [usize; 5];

#[derive(Serialize, Debug)]
pub struct Stats {
    category: &'static str,
    num_samples: usize,
    min: f64,
    max: f64,
    sum: f64,
    average: f64,
    bytes: f64,
    bandwidth: f64,
    histogram: Histogram,
}

impl Stats {
    pub fn new(category: &'static str, mut samples: Vec<f64>, obj_size: usize) -> Stats {
        if samples.is_empty() {
            panic!("supplied samples vec is empty");
        }
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = samples.first().unwrap();
        let max = samples.last().unwrap();
        let sum : f64 = samples.iter().sum();
        let bytes = samples.len() as f64 * obj_size as f64;
        let average = sum / samples.len() as f64;
        let bandwidth = bytes / sum as f64;
        let mut hist : Histogram = [0.0; 5];
        let mut counts : HistCounts = [0; 5];
        counts[FIFTY] = (samples.len() as f64 * 0.5) as usize;
        counts[SEVENTY] = (samples.len() as f64 * 0.7) as usize;
        counts[NINETY] = (samples.len() as f64 * 0.9) as usize;
        counts[NINETY5] = (samples.len() as f64 * 0.95) as usize;
        counts[NINETY9] = (samples.len() as f64 * 0.99) as usize;
        hist[FIFTY] = samples[counts[FIFTY]];
        hist[SEVENTY] = samples[counts[SEVENTY]];
        hist[NINETY] = samples[counts[NINETY]];
        hist[NINETY5] = samples[counts[NINETY5]];
        hist[NINETY9] = samples[counts[NINETY9]];

        Stats {
            category,
            min: *min,
            max: *max,
            sum,
            average,
            bytes,
            num_samples: samples.len(),
            bandwidth,
            histogram: hist,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::ser::to_string(self).unwrap()
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter) -> FResult {
        writeln!(f, "Timing results for {}", self.category);
        writeln!(f, "Bandwidth = {:.3} MB/sec", self.bytes / self.sum as f64 / (1024.0 * 1024.0));
        writeln!(f, "Mean = {:.3}", (self.max - self.min) / 2_f64);
        writeln!(f, "Average = {:.3}", self.sum / self.num_samples as f64);
        writeln!(f, "Max = {:.3}", self.max);
        writeln!(f, "Min = {:.3}", self.min);
        writeln!(f, "50th Percenile value = {:.3}", self.histogram[FIFTY]);
        writeln!(f, "70th Percenile value = {:.3}", self.histogram[SEVENTY]);
        writeln!(f, "90th Percenile value = {:.3}", self.histogram[NINETY]);
        writeln!(f, "95th Percenile value = {:.3}", self.histogram[NINETY5]);
        writeln!(f, "99th Percenile value = {:.3}", self.histogram[NINETY9])
    }
}

