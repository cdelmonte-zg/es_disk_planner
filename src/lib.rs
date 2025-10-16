#![doc = include_str!("../README.md")]

pub mod planner {
    use std::fmt::{Display, Formatter, Result as FmtResult};

    /// Represents the computed capacity plan for an Elasticsearch cluster.
    ///
    /// All values are expressed in **gigabytes (GB, base-10)**.
    /// This struct is returned by the capacity calculation function and
    /// provides both cluster-level and per-node estimates.
    #[derive(Debug, Clone, Copy)]
    pub struct Plan {
        /// Total data size for all primary and replica shards combined.
        ///
        /// Formula: `primaries * shard_size_gb * (1 + replicas)`
        pub base: f64,

        /// Base size plus Lucene merge overhead.
        ///
        /// Formula: `base * (1 + overhead_merge)`
        pub with_merge: f64,

        /// Size after applying headroom for watermarks and ingestion bursts.
        ///
        /// Formula: `with_merge * (1 + headroom)`
        pub with_headroom: f64,

        /// Total relocation/rebalancing buffer for all nodes combined.
        ///
        /// Formula: `buffer_per_node_gb * nodes`
        pub buffer_total: f64,

        /// Total cluster disk requirement, including overhead, headroom, and buffer.
        ///
        /// Formula: `with_headroom + buffer_total`
        pub total_cluster: f64,

        /// Recommended data size per node, averaged across the cluster.
        ///
        /// Formula: `total_cluster / nodes`
        pub per_node: f64,

        /// Recommended physical disk size per node to stay below the target utilization.
        ///
        /// Formula: `per_node / target_utilization`
        pub disk_per_node: f64,

        // --- Inputs echoed for reporting ---
        /// Target maximum disk utilization ratio (e.g. 0.75 = 75%).
        pub target_utilization: f64,
        /// Number of data nodes in the cluster.
        pub nodes: u32,
        /// Total number of primary shards.
        pub primaries: u32,
        /// Number of replica shards per primary.
        pub replicas: u32,
        /// Average shard size in GB (base-10).
        pub shard_size_gb: f64,
        /// Merge overhead fraction (e.g. 0.2 = 20%).
        pub overhead_merge: f64,
        /// Headroom fraction (e.g. 0.3 = 30%).
        pub headroom: f64,
        /// Optional relocation buffer per node in GB (defaults to shard size if `None`).
        pub buffer_per_node_gb: Option<f64>,
    }

    /// Computes an estimated disk capacity plan for an Elasticsearch cluster.
    ///
    /// This function applies a simplified model to estimate how much **disk space**
    /// (in gigabytes, base-10) is required across the entire cluster and per node.
    ///
    /// # Parameters
    ///
    /// - `nodes` — Number of data nodes in the cluster. Must be greater than zero.
    /// - `primaries` — Total number of primary shards across all indices.
    /// - `replicas` — Number of replicas for each primary shard.
    /// - `shard_size_gb` — Average size of a single shard, in gigabytes (GB).
    /// - `overhead_merge` — Fractional overhead for Lucene segment merges (e.g. `0.2` = 20%).
    /// - `headroom` — Fractional safety margin for disk watermarks and ingestion bursts (e.g. `0.3` = 30%).
    /// - `buffer_per_node_gb` — Optional relocation/rebalancing buffer per node.  
    ///   If `None`, defaults to `shard_size_gb`.
    /// - `target_utilization` — Desired maximum disk utilization ratio (e.g. `0.75` = 75%).  
    ///   Must be within the range `(0, 1]`.
    ///
    /// # Returns
    ///
    /// On success, returns a [`Plan`] struct containing all intermediate and final
    /// capacity estimates (cluster-level and per-node).
    ///
    /// # Errors
    ///
    /// Returns an [`Err`] string if:
    ///
    /// - `nodes` is `0`
    /// - `target_utilization` ≤ `0.0` or > `1.0`
    /// - `overhead_merge` or `headroom` < `0.0`
    /// - `shard_size_gb` ≤ `0.0`
    ///
    /// # Formulas
    ///
    /// ```text
    /// base = primaries * shard_size_gb * (1 + replicas)
    /// with_merge = base * (1 + overhead_merge)
    /// with_headroom = with_merge * (1 + headroom)
    /// buffer_total = buffer_per_node_gb * nodes
    /// total_cluster = with_headroom + buffer_total
    /// per_node = total_cluster / nodes
    /// disk_per_node = per_node / target_utilization
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use es_disk_planner::{plan_capacity, Plan};
    ///
    /// let plan = plan_capacity(5, 10, 1, 50.0, 0.20, 0.30, None, 0.75).unwrap();
    ///
    /// assert!((plan.total_cluster - 1810.0).abs() < 1e-6);
    /// assert!((plan.disk_per_node - 482.7).abs() < 0.1);
    /// ```
    ///
    /// # Notes
    ///
    /// - All calculations use **decimal gigabytes (GB)**, not GiB (1024-based).
    /// - The model follows Elastic’s general sizing guidelines
    ///   (20–50 GB per shard, ≤ 30 GB JVM heap, ≥ 64 GB node RAM).
    ///
    /// # See Also
    ///
    /// [`Plan`] — The struct containing the computed results.
    #[allow(clippy::too_many_arguments)]
    pub fn plan_capacity(
        nodes: u32,
        primaries: u32,
        replicas: u32,
        shard_size_gb: f64,
        overhead_merge: f64,
        headroom: f64,
        buffer_per_node_gb: Option<f64>,
        target_utilization: f64,
    ) -> Result<Plan, String> {
        if nodes == 0 {
            return Err("nodes must be > 0".into());
        }
        if target_utilization <= 0.0 || target_utilization > 1.0 {
            return Err("target_utilization must be in (0, 1]".into());
        }
        if overhead_merge < 0.0 || headroom < 0.0 {
            return Err("overhead_merge/headroom must be >= 0".into());
        }
        if shard_size_gb <= 0.0 {
            return Err("shard_size_gb must be > 0".into());
        }

        let nodes_f = nodes as f64;
        let primaries_f = primaries as f64;
        let replicas_f = replicas as f64;

        let buf = buffer_per_node_gb.unwrap_or(shard_size_gb);

        let base = primaries_f * shard_size_gb * (1.0 + replicas_f);

        let with_merge = base * (1.0 + overhead_merge);

        let with_headroom = with_merge * (1.0 + headroom);

        let buffer_total = buf * nodes_f;

        let total_cluster = with_headroom + buffer_total;

        let per_node = total_cluster / nodes_f;

        let disk_per_node = per_node / target_utilization;

        Ok(Plan {
            base,
            with_merge,
            with_headroom,
            buffer_total,
            total_cluster,
            per_node,
            disk_per_node,
            target_utilization,
            nodes,
            primaries,
            replicas,
            shard_size_gb,
            overhead_merge,
            headroom,
            buffer_per_node_gb,
        })
    }

    fn fmt_gb(x: f64) -> String {
        format!("{:.1} GB", x)
    }
    fn fmt_tb(x: f64) -> String {
        format!("{:.2} TB", x / 1000.0)
    }

    impl Display for Plan {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            writeln!(f, "=== Elasticsearch Disk Capacity Planner ===")?;
            writeln!(f, "Nodes: {}", self.nodes)?;
            writeln!(f, "Primary shards: {}", self.primaries)?;
            writeln!(f, "Replicas per shard: {}", self.replicas)?;
            writeln!(
                f,
                "Shard size: {} | Overhead merge: {:.0}% | Headroom: {:.0}%",
                fmt_gb(self.shard_size_gb),
                self.overhead_merge * 100.0,
                self.headroom * 100.0
            )?;
            writeln!(
                f,
                "Relocation buffer per node: {}",
                fmt_gb(self.buffer_per_node_gb.unwrap_or(self.shard_size_gb))
            )?;
            writeln!(
                f,
                "Target disk utilization: {:.0}%",
                self.target_utilization * 100.0
            )?;
            writeln!(f)?;

            writeln!(
                f,
                "Base (primaries+replicas): {} ({})",
                fmt_gb(self.base),
                fmt_tb(self.base)
            )?;
            writeln!(
                f,
                "+ Merge overhead:         {} ({})",
                fmt_gb(self.with_merge),
                fmt_tb(self.with_merge)
            )?;
            writeln!(
                f,
                "+ Headroom:               {} ({})",
                fmt_gb(self.with_headroom),
                fmt_tb(self.with_headroom)
            )?;
            writeln!(
                f,
                "+ Total buffer:           {} ({})",
                fmt_gb(self.buffer_total),
                fmt_tb(self.buffer_total)
            )?;
            writeln!(
                f,
                "= Cluster total:          {} ({})",
                fmt_gb(self.total_cluster),
                fmt_tb(self.total_cluster)
            )?;
            writeln!(f)?;
            writeln!(
                f,
                "Per node (recommended):   {} ({})",
                fmt_gb(self.per_node),
                fmt_tb(self.per_node)
            )?;
            writeln!(
                f,
                "Disk per node (<~{:.0}%): {} ({})",
                self.target_utilization * 100.0,
                fmt_gb(self.disk_per_node),
                fmt_tb(self.disk_per_node)
            )?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    // Scenario: 5 nodi, 10 primari, 1 replica, shard=50GB, overhead=20%, headroom=30%, buffer=default(=50GB), target=0.75
    #[test]
    fn example_numbers_match() {
        let p = plan_capacity(5, 10, 1, 50.0, 0.20, 0.30, None, 0.75).unwrap();
        assert!((p.base - 1000.0).abs() < 1e-6);
        assert!((p.with_merge - 1200.0).abs() < 1e-6);
        assert!((p.with_headroom - 1560.0).abs() < 1e-6);
        assert!((p.buffer_total - 250.0).abs() < 1e-6);
        assert!((p.total_cluster - 1810.0).abs() < 1e-6);
        assert!((p.per_node - 362.0).abs() < 1e-6);
        assert!((p.disk_per_node - 482.6666667).abs() < 1e-3);
    }

    #[test]
    fn rejects_bad_utilization() {
        assert!(plan_capacity(5, 10, 1, 50.0, 0.2, 0.3, None, 0.0).is_err());
        assert!(plan_capacity(5, 10, 1, 50.0, 0.2, 0.3, None, 1.01).is_err());
    }

    #[test]
    fn custom_buffer() {
        let p = plan_capacity(3, 6, 1, 40.0, 0.1, 0.2, Some(80.0), 0.8).unwrap();
        // base = 6*40*(1+1)=480; with_merge=528; with_headroom=633.6; buffer_total=80*3=240; total=873.6
        assert!((p.total_cluster - 873.6).abs() < 1e-6);
        // per_node = 291.2; disk_per_node = 291.2/0.8 = 364
        assert!((p.disk_per_node - 364.0).abs() < 1e-6);
    }
}

pub use planner::{plan_capacity, Plan};