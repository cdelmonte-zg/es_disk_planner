# Elasticsearch Disk Capacity Planner

This tool estimates the **disk capacity requirements** for an Elasticsearch cluster,
taking into account:

* Primary and replica shards
* Lucene merge overhead
* Headroom for operational safety (disk watermarks and ingestion bursts)
* Relocation buffer per node
* Target maximum disk utilization

The output helps plan realistic disk sizes per node and total cluster capacity.

---

## 🔢 Calculation Model

```text
base = primaries * shard_size_gb * (1 + replicas)
with_merge = base * (1 + overhead_merge)
with_headroom = with_merge * (1 + headroom)
buffer_total = buffer_per_node_gb * nodes
total_cluster = with_headroom + buffer_total
per_node = total_cluster / nodes
disk_per_node = per_node / target_utilization
```

---

### Parameters

| Parameter              | Default           | Description                                                     |
| ---------------------- | ----------------- | --------------------------------------------------------------- |
| `--shard_size_gb`      | `50`              | Average size of a single shard on disk (compressed Lucene data) |
| `--overhead_merge`     | `0.20` (20%)      | Temporary space required by Lucene segment merges               |
| `--headroom`           | `0.30` (30%)      | Safety margin to stay below disk watermarks (85–90%)            |
| `--buffer_per_node_gb` | = `shard_size_gb` | Space reserved per node for shard relocation/rebalancing        |
| `--target_utilization` | `0.75` (75%)      | Maximum desired disk usage ratio                                |

---

## 📊 Example Output

```text
=== Elasticsearch Disk Capacity Planner ===
Nodes: 5
Primary shards: 10
Replicas per shard: 1
Shard size: 50.0 GB | Overhead merge: 20% | Headroom: 30%
Relocation buffer per node: 50.0 GB
Target disk utilization: 75%

Base (primaries+replicas): 1000.0 GB (1.00 TB)
+ Overhead merge:        1200.0 GB (1.20 TB)
+ Headroom:              1560.0 GB (1.56 TB)
+ Total buffer:          250.0 GB  (0.25 TB)
= Cluster total:         1810.0 GB (1.81 TB)

Per node (recommended):  362.0 GB (0.36 TB)
Disk per node @ <75%:    482.7 GB (0.48 TB)
```

---

## 💡 Interpretation

* **Base (primaries + replicas)** — total indexed data size on disk.
* **Merge overhead** — extra space required during Lucene segment merges.
* **Headroom** — operational slack to avoid hitting high/flood-stage watermarks.
* **Buffer per node** — space required to receive the largest shard during relocation.
* **Target utilization** — desired maximum disk usage (usually 70–80%).

This model provides an approximate but **operationally safe** estimation for
capacity planning in Elasticsearch clusters.

---

## 🧮 Example Scenario

* 5 data nodes
* 10 primary shards
* 1 replica per shard
* 50 GB average shard size
* 20% merge overhead
* 30% headroom
* 50 GB relocation buffer per node
* 75% target disk utilization

**Result:** ~**1.8 TB total cluster capacity**, or **~480–500 GB per node** to stay below 75% usage.

---

## ⚙️ Operational Notes

* The results refer to **disk usage**, *not* JVM heap or RAM.
* Typical Elasticsearch node sizing guidelines:

  * JVM heap ≤ **30 GB**
  * Node memory ≥ **64 GB** (≈ 50% heap, 50% OS file cache)
  * Shard size **20–50 GB**
* The model aligns with Elastic’s published best practices.

---

## ⚠️ Limitations

* Assumes uniform shard sizes and compression ratios.
* Does not include local snapshots or external repository overhead.
* Does not model “cold” or “frozen” tiers.
* Merge and headroom factors are static (simplified estimation).

---

## 🧰 Usage Examples

```bash
# Default example
cargo run -- \
  --nodes 5 \
  --primaries 10 \
  --replicas 1 \
  --shard_size_gb 50 \
  --overhead_merge 0.20 \
  --headroom 0.30 \
  --target_utilization 0.75

# Two replicas, larger shards
cargo run -- --nodes 5 --primaries 10 --replicas 2 --shard_size_gb 80

# Conservative watermarks
cargo run -- --nodes 6 --headroom 0.4 --target_utilization 0.65
```

---

## 🧩 Future Improvements

* Add `--units iec` to switch between GB/TB (1000-based) and GiB/TiB (1024-based)
* Support JSON/CSV output for pipeline integration
* Optional retrieval of real shard stats from `_cat/shards` via REST API
* Integrate into a monitoring workflow for continuous capacity validation
