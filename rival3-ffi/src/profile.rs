#[repr(C)]
#[derive(Clone, Copy)]
pub struct RivalExecution {
    pub instruction_idx: i32,
    pub precision: u32,
    pub time_ms: f64,
    pub iteration: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RivalAggregatedProfile {
    pub instruction_idx: i32,
    pub precision_bucket: u32,
    pub total_time_ms: f64,
    pub count: usize,
}

#[repr(C)]
pub struct RivalProfileSummary {
    pub entries: *const RivalAggregatedProfile,
    pub len: usize,
    pub bumps: u32,
    pub iterations: u32,
}

pub(crate) struct ProfileCache {
    pub executions: Vec<RivalExecution>,
    pub aggregated: Vec<RivalAggregatedProfile>,
    pub last_bumps: u32,
    pub last_iterations: u32,
}

impl ProfileCache {
    pub fn new() -> Self {
        Self {
            executions: Vec::new(),
            aggregated: Vec::new(),
            last_bumps: 0,
            last_iterations: 0,
        }
    }

    pub fn aggregate_from(
        &mut self,
        records: &[rival::Execution],
        bucket_size: u32,
        bumps: usize,
        iterations: usize,
    ) -> RivalProfileSummary {
        let bucket_size = bucket_size.max(1);

        self.aggregated.clear();
        self.aggregated.reserve(records.len());
        for exec in records {
            self.aggregated.push(RivalAggregatedProfile {
                instruction_idx: exec.number,
                precision_bucket: exec.precision - (exec.precision % bucket_size),
                total_time_ms: exec.time_ms,
                count: 1,
            });
        }
        self.aggregated
            .sort_by_key(|e| (e.instruction_idx, e.precision_bucket));
        self.aggregated.dedup_by(|later, earlier| {
            let same = later.instruction_idx == earlier.instruction_idx
                && later.precision_bucket == earlier.precision_bucket;
            if same {
                earlier.total_time_ms += later.total_time_ms;
                earlier.count += later.count;
            }
            same
        });

        self.last_bumps = bumps as u32;
        self.last_iterations = iterations as u32;

        RivalProfileSummary {
            entries: self.summary_ptr(),
            len: self.aggregated.len(),
            bumps: self.last_bumps,
            iterations: self.last_iterations,
        }
    }

    pub fn summary_from_cache(&self) -> RivalProfileSummary {
        RivalProfileSummary {
            entries: self.summary_ptr(),
            len: self.aggregated.len(),
            bumps: self.last_bumps,
            iterations: self.last_iterations,
        }
    }

    pub fn cache_executions<'a, I>(&mut self, records: I)
    where
        I: Iterator<Item = &'a rival::Execution>,
    {
        self.executions.clear();
        for exec in records {
            self.executions.push(RivalExecution {
                instruction_idx: exec.number,
                precision: exec.precision,
                time_ms: exec.time_ms,
                iteration: exec.iteration.min(u32::MAX as usize) as u32,
            });
        }
    }

    fn summary_ptr(&self) -> *const RivalAggregatedProfile {
        if self.aggregated.is_empty() {
            std::ptr::null()
        } else {
            self.aggregated.as_ptr()
        }
    }
}
