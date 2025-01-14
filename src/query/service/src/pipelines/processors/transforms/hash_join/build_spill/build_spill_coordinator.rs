// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use common_base::base::tokio::sync::Notify;
use common_exception::Result;
use common_expression::DataBlock;
use parking_lot::RwLock;

/// Coordinate all hash join build processors to spill.
/// It's shared by all hash join build processors.
/// When hash join build needs to spill, all processor will stop executing and prepare to spill.
/// The last one will be as the coordinator to spill all processors and then wake up all processors to continue executing.
pub struct BuildSpillCoordinator {
    /// Need to spill, if one of the builders need to spill, this flag will be set to true.
    need_spill: AtomicBool,
    /// Current waiting spilling processor count.
    pub(crate) waiting_spill_count: RwLock<usize>,
    /// Total processor count.
    pub(crate) total_builder_count: usize,
    /// Notify all waiting spilling processors to start spill.
    pub(crate) notify_spill: Arc<Notify>,
    /// Spill tasks, the size is the same as the total active processor count.
    pub(crate) spill_tasks: RwLock<VecDeque<Vec<(u8, DataBlock)>>>,
    /// If send partition set to probe
    pub send_partition_set: AtomicBool,
    /// When a build processor won't trigger spill, the field will plus one
    pub non_spill_processors: RwLock<usize>,
}

impl BuildSpillCoordinator {
    pub fn create(total_builder_count: usize) -> Arc<Self> {
        Arc::new(Self {
            need_spill: AtomicBool::new(false),
            waiting_spill_count: RwLock::new(0),
            total_builder_count,
            notify_spill: Arc::new(Default::default()),
            spill_tasks: Default::default(),
            send_partition_set: Default::default(),
            non_spill_processors: Default::default(),
        })
    }

    // Start to spill.
    pub(crate) fn notify_spill(&self) {
        self.notify_spill.notify_waiters();
    }

    // Called by hash join build processor, if current processor need to spill, then set `need_spill` to true.
    pub fn need_spill(&self) -> Result<()> {
        self.need_spill.store(true, Ordering::SeqCst);
        Ok(())
    }

    // If current waiting spilling builder is the last one, then spill all builders.
    pub(crate) fn wait_spill(&self) -> Result<bool> {
        let non_spill_processors = self.non_spill_processors.read();
        let mut waiting_spill_count = self.waiting_spill_count.write();
        *waiting_spill_count += 1;
        if *waiting_spill_count == (self.total_builder_count - *non_spill_processors) {
            // Reset waiting_spill_count
            *waiting_spill_count = 0;
            // No need to wait spill, the processor is the last one
            return Ok(false);
        }
        Ok(true)
    }

    // Get the need_spill flag.
    pub fn get_need_spill(&self) -> bool {
        self.need_spill.load(Ordering::SeqCst)
    }

    // Set the need_spill flag to false.
    pub fn no_need_spill(&self) {
        self.need_spill.store(false, Ordering::SeqCst);
    }

    // Wait for notify to spill
    #[async_backtrace::framed]
    pub async fn wait_spill_notify(&self) {
        self.notify_spill.notified().await
    }
}
