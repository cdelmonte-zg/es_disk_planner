use clap::Parser;
use es_disk_planner::plan_capacity;

/// Command-line arguments for the Elasticsearch Disk Capacity Planner.
#[derive(Debug, Parser)]
struct Args {
    /// Number of data nodes in the cluster.
    #[arg(long)]
    nodes: u32,

    /// Total number of primary shards (sum across all indices).
    #[arg(long)]
    primaries: u32,

    /// Number of replica shards per primary.
    #[arg(long, default_value_t = 1)]
    replicas: u32,

    /// Average size of a single shard in gigabytes (base-10 GB).
    #[arg(long, default_value_t = 50.0)]
    shard_size_gb: f64,

    /// Additional temporary space required for Lucene segment merges (fraction, e.g. 0.2 = 20%).
    #[arg(long, default_value_t = 0.20)]
    overhead_merge: f64,

    /// Operational headroom for disk watermarks and ingestion bursts (fraction, e.g. 0.3 = 30%).
    #[arg(long, default_value_t = 0.30)]
    headroom: f64,

    /// Extra buffer per node (in GB) reserved for shard relocation and rebalancing.
    /// If omitted, defaults to `shard_size_gb`.
    #[arg(long)]
    buffer_per_node_gb: Option<f64>,

    /// Maximum desired disk utilization ratio per node (e.g. 0.75 = keep usage below ~75%).
    #[arg(long, default_value_t = 0.75)]
    target_utilization: f64,
}

fn main() {
    let a = Args::parse();

    match plan_capacity(
        a.nodes,
        a.primaries,
        a.replicas,
        a.shard_size_gb,
        a.overhead_merge,
        a.headroom,
        a.buffer_per_node_gb,
        a.target_utilization,
    ) {
        Ok(plan) => println!("{}", plan),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    }
}
