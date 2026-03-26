//! Helpers for measuring and reporting contract execution costs.

/// A report of the compute costs for a contract invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostReport {
    instructions: u64,
    memory: u64,
}

impl CostReport {
    /// Creates a new cost report.
    pub fn new(instructions: u64, memory: u64) -> Self {
        Self {
            instructions,
            memory,
        }
    }

    /// Returns the number of CPU instructions consumed.
    pub fn instructions(&self) -> u64 {
        self.instructions
    }

    /// Returns the peak memory usage in bytes.
    pub fn memory_bytes(&self) -> u64 {
        self.memory
    }

    /// Returns the estimated network fee in stroops.
    ///
    /// This is a simplified estimation based on instructions.
    pub fn fee_stroops(&self) -> i64 {
        // Simple heuristic: 100 instructions = 1 stroop
        // This should ideally be calibrated to match the network protocol.
        (self.instructions / 100) as i64
    }

    /// Returns a human-readable report of the costs.
    pub fn report(&self) -> String {
        format!(
            "Cost Report:\n  Instructions: {}\n  Memory:       {} bytes\n  Est. Fee:     {} stroops",
            self.instructions,
            self.memory,
            self.fee_stroops()
        )
    }
}
