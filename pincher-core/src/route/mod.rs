//! # Route Module
//!
//! Room routing and decision-making using ternary-valued edge weights.
//!
//! This module provides a [`RoomGraph`] type for modeling room-to-room
//! relationships as a ternary-weighted graph where edges carry one of
//! three weights via [`ternary_types::Ternary`]:
//!
//! | Variant          | Value | Meaning                              |
//! |------------------|-------|--------------------------------------|
//! | `Positive`       | `+1`  | Trusted connection, preferred route   |
//! | `Neutral`        | `0`   | No relationship, neutral              |
//! | `Negative`       | `−1`  | Adversarial/blocked path, avoid       |
//!
//! ## Example
//!
//! ```rust,ignore
//! use pincher_core::route::RoomGraph;
//!
//! let mut g = RoomGraph::new(3);
//! g.add_edge(0, 1, ternary_types::Ternary::Positive);
//! g.add_edge(1, 2, ternary_types::Ternary::Positive);
//!
//! let dist = g.distances_from(0);
//! assert_eq!(dist[0], Some(0.0));
//! assert_eq!(dist[2], Some(2.0));
//! ```

use std::collections::VecDeque;
use ternary_types::Ternary;

// ── Ternary-Weighted Graph ──────────────────────────────────────────

/// A simple ternary-weighted adjacency-list graph.
///
/// Edge weights are [`Ternary`] values (`−1`, `0`, `+1`). Supports
/// shortest-path queries via Bellman-Ford, community detection via label
/// propagation, and spectral clustering.
#[derive(Clone, Debug)]
pub struct TernaryGraph {
    n: usize,
    pub directed: bool,
    adj: Vec<Vec<(usize, Ternary)>>,
}

impl TernaryGraph {
    /// Create a new graph with `n` nodes.
    ///
    /// Set `directed = true` for one-way edges (recommended when
    /// using `Negative` edges to avoid automatic negative cycles).
    pub fn new(n: usize, directed: bool) -> Self {
        TernaryGraph {
            n,
            directed,
            adj: vec![vec![]; n],
        }
    }

    /// Add a ternary-weighted edge from `u` to `v`.
    ///
    /// In undirected mode, inserts the reverse edge automatically.
    pub fn add_edge(&mut self, u: usize, v: usize, w: Ternary) {
        self.adj[u].push((v, w));
        if !self.directed {
            self.adj[v].push((u, w));
        }
    }

    /// Return the numeric weight of the edge from `u` to `v`, if one exists.
    ///
    /// Only returns the *first* edge — use [`neighbors`] for all incident edges.
    pub fn edge_weight(&self, u: usize, v: usize) -> Option<f64> {
        self.adj[u]
            .iter()
            .find(|&&(n, _)| n == v)
            .map(|&(_, t)| i8::from(t) as f64)
    }

    /// Neighbors of node `u` with their edge weights.
    pub fn neighbors(&self, u: usize) -> &[(usize, Ternary)] {
        &self.adj[u]
    }

    /// Number of incident edges for node `u`.
    pub fn degree(&self, u: usize) -> usize {
        self.adj[u].len()
    }

    /// Total number of nodes.
    pub fn node_count(&self) -> usize {
        self.n
    }
}

// ── Graph Algorithms ────────────────────────────────────────────────

/// Shortest paths from `source` to all other nodes using Bellman-Ford.
///
/// Handles negative edge weights. Nodes reachable through a negative
/// cycle return `None`.
pub fn shortest_paths(g: &TernaryGraph, source: usize) -> Vec<Option<f64>> {
    let n = g.node_count();
    let mut dist = vec![f64::INFINITY; n];
    dist[source] = 0.0;

    // Relax all edges n-1 times
    for _ in 0..n - 1 {
        for u in 0..n {
            for &(v, w) in &g.adj[u] {
                let weight = i8::from(w) as f64;
                if dist[u] != f64::INFINITY && dist[u] + weight < dist[v] {
                    dist[v] = dist[u] + weight;
                }
            }
        }
    }

    // Check for negative cycles: if any edge can still be relaxed, mark
    // the node as unreachable (None).
    let mut in_neg_cycle = vec![false; n];
    for u in 0..n {
        for &(v, w) in &g.adj[u] {
            let weight = i8::from(w) as f64;
            if dist[u] != f64::INFINITY && dist[u] + weight < dist[v] {
                in_neg_cycle[v] = true;
            }
        }
    }

    // Propagate negative cycles using DFS/BFS
    // Bellman-Ford only guarantees detection by the first relaxation of a
    // negative-cycle node, so we propagate transitive closure.
    let mut q: VecDeque<usize> = (0..n).filter(|&i| in_neg_cycle[i]).collect();
    while let Some(u) = q.pop_front() {
        for &(v, _) in &g.adj[u] {
            if !in_neg_cycle[v] {
                in_neg_cycle[v] = true;
                q.push_back(v);
            }
        }
    }

    (0..n)
        .map(|i| {
            if in_neg_cycle[i] || dist[i] == f64::INFINITY {
                None
            } else {
                Some(dist[i])
            }
        })
        .collect()
}

/// All-pairs shortest paths via Floyd-Warshall.
pub fn all_pairs_shortest_paths(g: &TernaryGraph) -> Vec<Vec<Option<f64>>> {
    let n = g.node_count();
    // Adjacency matrix initialised to INF
    let mut dist = vec![vec![f64::INFINITY; n]; n];
    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        dist[i][i] = 0.0;
        for &(j, w) in &g.adj[i] {
            let weight = i8::from(w) as f64;
            if weight < dist[i][j] {
                dist[i][j] = weight;
            }
        }
    }

    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if dist[i][k] != f64::INFINITY && dist[k][j] != f64::INFINITY {
                    let nd = dist[i][k] + dist[k][j];
                    if nd < dist[i][j] {
                        dist[i][j] = nd;
                    }
                }
            }
        }
    }

    // Convert — INF becomes None
    dist.iter()
        .map(|row| {
            row.iter()
                .map(|&d| {
                    if d == f64::INFINITY || d.is_nan() || d == f64::NEG_INFINITY {
                        None
                    } else {
                        Some(d)
                    }
                })
                .collect()
        })
        .collect()
}

/// Label propagation community detection (signed).
///
/// Positive edges attract (same community), negative edges repel (different
/// community). `max_iters` caps the number of iterations.
pub fn label_propagation(g: &TernaryGraph, max_iters: usize) -> Vec<usize> {
    let n = g.node_count();
    let mut labels: Vec<usize> = (0..n).collect();

    for _iter in 0..max_iters {
        let mut changed = false;

        // Shuffle nodes for randomness (batch-order is fine for determinism)
        for u in 0..n {
            let mut scores: Vec<f64> = vec![0.0; n];

            for &(v, w) in &g.adj[u] {
                let weight = i8::from(w) as f64;
                scores[labels[v]] += weight;
            }

            // Pick the label with the highest score (break ties by smaller id)
            let best = scores
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some((best_label, _)) = best {
                if labels[u] != best_label {
                    labels[u] = best_label;
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    labels
}

/// Connected components using only positive edges.
pub fn connected_components(g: &TernaryGraph) -> Vec<usize> {
    let n = g.node_count();
    let mut comp = vec![usize::MAX; n];
    let mut next_id = 0;

    for start in 0..n {
        if comp[start] != usize::MAX {
            continue;
        }

        // BFS on positive edges only
        let mut q = VecDeque::new();
        q.push_back(start);
        comp[start] = next_id;

        while let Some(u) = q.pop_front() {
            for &(v, w) in &g.adj[u] {
                if w == Ternary::Positive && comp[v] == usize::MAX {
                    comp[v] = next_id;
                    q.push_back(v);
                }
            }
        }

        next_id += 1;
    }

    comp
}

/// Spectral clustering using power iteration on the signed Laplacian.
///
/// Returns a vector of cluster assignments for `k` clusters via
/// `f64` k-means on the Laplacian eigenvector embedding.
pub fn spectral_clustering(g: &TernaryGraph, k: usize) -> Vec<usize> {
    let n = g.node_count();
    if n == 0 || k == 0 {
        return vec![0; n];
    }
    if k >= n {
        return (0..n).collect();
    }

    let lap = laplacian(g);
    // Find the smallest eigenvalues via power iteration on the shifted matrix
    // (shift = identity * max_eigenvalue_estimate to make it positive-definite)
    let max_eig = power_iteration_max(&lap, 100);
    let shift = max_eig * 1.1 + 1.0;

    // Invert spectrum: compute eigenvectors of (shift * I - L) = largest eig of
    // (shift * I - L) correspond to smallest eig of L.
    let shifted: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    if i == j {
                        shift - lap[i][j]
                    } else {
                        -lap[i][j]
                    }
                })
                .collect()
        })
        .collect();

    // Get top-k eigenvectors via power iteration + deflation
    let mut eigenvecs: Vec<Vec<f64>> = Vec::new();
    let mut working = shifted.clone();

    for _ in 0..k.min(n) {
        let (eigvec, _) = power_iteration_eigenvec(&working, 100);
        let norm: f64 = eigvec.iter().map(|x| x * x).sum::<f64>().sqrt();
        let eigvec: Vec<f64> = if norm > 1e-12 {
            eigvec.iter().map(|x| x / norm).collect()
        } else {
            eigvec
        };
        eigenvecs.push(eigvec.clone());

        // Deflate
        for i in 0..n {
            for j in 0..n {
                working[i][j] -= eigvec[i] * eigvec[j];
            }
        }
    }

    // Build embedding matrix: row i = [eigvec1[i], eigvec2[i], ...]
    let embedding: Vec<Vec<f64>> = (0..n)
        .map(|i| eigenvecs.iter().map(|ev| ev[i]).collect())
        .collect();

    // Simple k-means on the embedding
    kmeans(&embedding, k, 100)
}

// ── Linear Algebra Helpers ──────────────────────────────────────────

/// Power iteration to estimate the largest eigenvalue.
fn power_iteration_max(mat: &[Vec<f64>], iters: usize) -> f64 {
    let n = mat.len();
    if n == 0 {
        return 0.0;
    }
    let mut v: Vec<f64> = (0..n).map(|i| (i as f64 + 1.0).sqrt()).collect();

    for _ in 0..iters {
        let mut w = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                w[i] += mat[i][j] * v[j];
            }
        }
        let norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-12 {
            for x in &mut w {
                *x /= norm;
            }
        }
        v = w;
    }

    // Rayleigh quotient
    let mut top = 0.0;
    let mut bottom = 0.0;
    for i in 0..n {
        for j in 0..n {
            top += v[i] * mat[i][j] * v[j];
        }
        bottom += v[i] * v[i];
    }
    top / bottom.abs().max(1e-12)
}

/// Power iteration to get dominant eigenvector and eigenvalue.
fn power_iteration_eigenvec(mat: &[Vec<f64>], iters: usize) -> (Vec<f64>, f64) {
    let n = mat.len();
    if n == 0 {
        return (vec![], 0.0);
    }
    let mut v: Vec<f64> = (0..n).map(|i| (i as f64 + 1.0).sqrt()).collect();

    for _ in 0..iters {
        let mut w = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                w[i] += mat[i][j] * v[j];
            }
        }
        let norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-12 {
            for x in &mut w {
                *x /= norm;
            }
        }
        v = w;
    }

    // Rayleigh quotient
    let mut eigval = 0.0;
    for i in 0..n {
        for j in 0..n {
            eigval += v[i] * mat[i][j] * v[j];
        }
    }
    let norm_sq: f64 = v.iter().map(|x| x * x).sum();
    eigval /= norm_sq.abs().max(1e-12);

    (v, eigval)
}

/// Simple k-means clustering.
fn kmeans(data: &[Vec<f64>], k: usize, max_iters: usize) -> Vec<usize> {
    let n = data.len();
    if n == 0 || k == 0 {
        return vec![0; n];
    }
    if k >= n {
        return (0..n).collect();
    }

    // Initialise centroids: use first k data points
    let mut centroids: Vec<Vec<f64>> = data.iter().take(k).cloned().collect();
    let mut assignments = vec![0usize; n];

    for _ in 0..max_iters {
        // Assign each point to nearest centroid
        let mut changed = false;
        for (i, point) in data.iter().enumerate() {
            let best = (0..k)
                .map(|c| {
                    let dist: f64 = point
                        .iter()
                        .zip(&centroids[c])
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum();
                    (c, dist)
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some((best_c, _)) = best {
                if assignments[i] != best_c {
                    assignments[i] = best_c;
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }

        // Recompute centroids
        let dim = data[0].len();
        let mut sums = vec![vec![0.0; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, point) in data.iter().enumerate() {
            let c = assignments[i];
            for (j, val) in point.iter().enumerate() {
                sums[c][j] += val;
            }
            counts[c] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                let inv = 1.0 / counts[c] as f64;
                for j in 0..dim {
                    centroids[c][j] = sums[c][j] * inv;
                }
            }
        }
    }

    assignments
}

// ── Matrix Construction ────────────────────────────────────────────

/// Signed Laplacian matrix (L = D - A).
pub fn laplacian(g: &TernaryGraph) -> Vec<Vec<f64>> {
    let n = g.node_count();
    let mut lap = vec![vec![0.0; n]; n];

    for (u, row) in lap.iter_mut().enumerate() {
        for &(v, w) in &g.adj[u] {
            let weight = i8::from(w) as f64;
            row[u] += weight.abs(); // degree = sum of |weight|
            row[v] -= weight; // −A_uv
        }
    }

    lap
}

/// Normalized signed Laplacian (D^(-1/2) * L * D^(-1/2)).
pub fn normalized_laplacian(g: &TernaryGraph) -> Vec<Vec<f64>> {
    let n = g.node_count();
    let lap = laplacian(g);
    let deg: Vec<f64> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| lap[i][j] * if i == j { 1.0 } else { -1.0 })
                .sum::<f64>()
        })
        .collect();

    let mut nl = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let di = if deg[i] > 1e-12 {
                1.0 / deg[i].sqrt()
            } else {
                1.0
            };
            let dj = if deg[j] > 1e-12 {
                1.0 / deg[j].sqrt()
            } else {
                1.0
            };
            // nl = D^(-1/2) * L * D^(-1/2)
            nl[i][j] = lap[i][j] * di * dj;
        }
    }
    nl
}

/// Adjacency matrix as f64 (Pos=1.0, Neg=-1.0, Zero=0.0).
pub fn adjacency_f64(g: &TernaryGraph) -> Vec<Vec<f64>> {
    let n = g.node_count();
    let mut adj = vec![vec![0.0; n]; n];
    for (u, row) in adj.iter_mut().enumerate() {
        for &(v, w) in &g.adj[u] {
            row[v] = i8::from(w) as f64;
        }
    }
    adj
}

/// Degree matrix (diagonal: sum of |weight| for each node).
pub fn degree_matrix(g: &TernaryGraph) -> Vec<Vec<f64>> {
    let n = g.node_count();
    let mut d = vec![vec![0.0; n]; n];
    for (u, row) in d.iter_mut().enumerate() {
        let sum_abs: f64 = g.adj[u]
            .iter()
            .map(|&(_, w)| i8::from(w).abs() as f64)
            .sum();
        row[u] = sum_abs;
    }
    d
}

/// Signed modularity score for a given community assignment.
pub fn modularity(g: &TernaryGraph, communities: &[usize]) -> f64 {
    let n = g.node_count();
    let mut total_pos_weight = 0.0;
    let mut total_neg_weight = 0.0;

    for u in 0..n {
        for &(_, w) in &g.adj[u] {
            let weight = i8::from(w) as f64;
            if weight > 0.0 {
                total_pos_weight += weight;
            } else if weight < 0.0 {
                total_neg_weight += weight.abs();
            }
        }
    }

    if total_pos_weight == 0.0 && total_neg_weight == 0.0 {
        return 0.0;
    }

    let m_plus = total_pos_weight / 2.0;
    let m_minus = total_neg_weight / 2.0;
    let m = m_plus + m_minus;

    let mut q = 0.0;
    for u in 0..n {
        let deg_pos: f64 = g.adj[u]
            .iter()
            .filter(|&&(_, w)| w == ternary_types::Ternary::Positive)
            .map(|_| 1.0)
            .sum();
        let deg_neg: f64 = g.adj[u]
            .iter()
            .filter(|&&(_, w)| w == ternary_types::Ternary::Negative)
            .map(|_| 1.0)
            .sum();

        for &(v, w) in &g.adj[u] {
            if communities[u] == communities[v] {
                let a_uv = i8::from(w) as f64;
                let expected_plus = if m_plus > 0.0 {
                    (deg_pos / (2.0 * m_plus)) * deg_pos
                } else {
                    0.0
                };
                let expected_minus = if m_minus > 0.0 {
                    (deg_neg / (2.0 * m_minus)) * deg_neg
                } else {
                    0.0
                };
                let expected = expected_plus - expected_minus;
                q += a_uv - expected;
            }
        }
    }

    if m > 0.0 {
        q / (2.0 * m)
    } else {
        0.0
    }
}

// ── RoomGraph ───────────────────────────────────────────────────────

/// A named room routing graph built on a [`TernaryGraph`].
///
/// Each room is a node with a name and optional agent list. The edges carry
/// ternary weights for routing decisions.
#[derive(Clone, Debug)]
pub struct Room {
    pub id: usize,
    pub name: String,
    pub agents: Vec<String>,
}

/// The top-level routing graph for pincher.
///
/// Wraps a [`TernaryGraph`] with room metadata so routes can be queried by name or id.
#[derive(Clone, Debug)]
pub struct RoomGraph {
    pub graph: TernaryGraph,
    pub rooms: Vec<Room>,
}

impl RoomGraph {
    /// Create a new room graph from a set of rooms (undirected).
    pub fn new(rooms: Vec<Room>) -> Self {
        let n = rooms.len();
        RoomGraph {
            graph: TernaryGraph::new(n, false),
            rooms,
        }
    }

    /// Convert to directed mode (edges become one-way).
    pub fn into_directed(mut self) -> Self {
        self.graph.directed = true;
        self
    }

    /// Add a trusted (positive-weight) route.
    pub fn add_trusted_route(&mut self, a: usize, b: usize) {
        self.graph.add_edge(a, b, ternary_types::Ternary::Positive);
    }

    /// Add a blocked (negative-weight) route.
    pub fn add_blocked_route(&mut self, a: usize, b: usize) {
        self.graph.add_edge(a, b, ternary_types::Ternary::Negative);
    }

    /// Shortest paths from `source` using Bellman-Ford.
    pub fn distances_from(&self, source: usize) -> Vec<Option<f64>> {
        shortest_paths(&self.graph, source)
    }

    /// Route cost from `source` to `target`.
    pub fn route_cost(&self, source: usize, target: usize) -> Option<f64> {
        let dist = self.distances_from(source);
        dist.get(target).copied().flatten()
    }

    /// All-pairs shortest paths.
    pub fn all_distances(&self) -> Vec<Vec<Option<f64>>> {
        all_pairs_shortest_paths(&self.graph)
    }

    /// Detect communities via label propagation.
    pub fn detect_communities(&self, max_iters: usize) -> Vec<usize> {
        label_propagation(&self.graph, max_iters)
    }

    /// Cluster into `k` groups via spectral clustering.
    pub fn cluster_rooms(&self, k: usize) -> Vec<usize> {
        spectral_clustering(&self.graph, k)
    }

    /// Signed modularity of a community assignment.
    pub fn community_modularity(&self, communities: &[usize]) -> f64 {
        modularity(&self.graph, communities)
    }

    /// Connected components using only positive edges.
    pub fn trusted_components(&self) -> Vec<usize> {
        connected_components(&self.graph)
    }

    /// Degree of a room.
    pub fn degree(&self, room: usize) -> usize {
        self.graph.degree(room)
    }

    /// Signed Laplacian matrix.
    pub fn laplacian(&self) -> Vec<Vec<f64>> {
        laplacian(&self.graph)
    }

    /// Normalized Laplacian.
    pub fn normalized_laplacian(&self) -> Vec<Vec<f64>> {
        normalized_laplacian(&self.graph)
    }

    /// Adjacency matrix as f64.
    pub fn adjacency(&self) -> Vec<Vec<f64>> {
        adjacency_f64(&self.graph)
    }

    /// Degree matrix.
    pub fn degree_matrix(&self) -> Vec<Vec<f64>> {
        degree_matrix(&self.graph)
    }
}

/// Build a simple routing graph from room names.
pub fn build_routing_graph(room_names: &[&str]) -> RoomGraph {
    let rooms: Vec<Room> = room_names
        .iter()
        .enumerate()
        .map(|(i, name)| Room {
            id: i,
            name: name.to_string(),
            agents: Vec::new(),
        })
        .collect();
    RoomGraph::new(rooms)
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortest_paths_simple() {
        let mut g = TernaryGraph::new(4, false);
        g.add_edge(0, 1, Ternary::Positive);
        g.add_edge(1, 2, Ternary::Positive);
        g.add_edge(2, 3, Ternary::Positive);

        let dist = shortest_paths(&g, 0);
        assert_eq!(dist[0], Some(0.0));
        assert_eq!(dist[1], Some(1.0));
        assert_eq!(dist[3], Some(3.0));
    }

    #[test]
    fn test_negative_edge_path() {
        let mut g = TernaryGraph::new(3, true);
        g.add_edge(0, 1, Ternary::Negative);
        g.add_edge(1, 2, Ternary::Positive);

        // 0 → 1: cost -1, 1 → 2: cost +1 => total 0
        let dist = shortest_paths(&g, 0);
        assert_eq!(dist[0], Some(0.0));
        assert_eq!(dist[1], Some(-1.0));
        assert_eq!(dist[2], Some(0.0));
    }

    #[test]
    fn test_negative_cycle_detection() {
        let mut g = TernaryGraph::new(2, false);
        g.add_edge(0, 1, Ternary::Negative);

        let dist = shortest_paths(&g, 0);
        assert_eq!(dist[1], None); // 0→1 = -1, 1→0 = -1, cycle = -2 → negative cycle
    }

    #[test]
    fn test_all_pairs_shortest_paths() {
        let mut g = TernaryGraph::new(3, false);
        g.add_edge(0, 1, Ternary::Positive);
        g.add_edge(1, 2, Ternary::Positive);

        let apsp = all_pairs_shortest_paths(&g);
        assert_eq!(apsp[0][2], Some(2.0));
        assert_eq!(apsp[2][0], Some(2.0));
    }

    #[test]
    fn test_label_propagation_basic() {
        let mut g = TernaryGraph::new(4, false);
        g.add_edge(0, 1, Ternary::Positive);
        g.add_edge(2, 3, Ternary::Positive);

        let labels = label_propagation(&g, 100);
        assert_eq!(labels[0], labels[1], "0 and 1 should be same community");
        assert_eq!(labels[2], labels[3], "2 and 3 should be same community");
    }

    #[test]
    fn test_connected_components() {
        let mut g = TernaryGraph::new(5, false);
        g.add_edge(0, 1, Ternary::Positive);
        g.add_edge(2, 3, Ternary::Positive);
        // Node 4 is isolated

        let comp = connected_components(&g);
        assert_eq!(comp[0], comp[1]);
        assert_eq!(comp[2], comp[3]);
        assert_ne!(comp[0], comp[2]);
        assert_ne!(comp[0], comp[4]);
        assert_ne!(comp[2], comp[4]);
    }

    #[test]
    fn test_room_graph_integration() {
        let mut rg = build_routing_graph(&["lobby", "dev", "staging"]);
        rg.add_trusted_route(0, 1);
        rg.add_trusted_route(1, 2);

        let dist = rg.distances_from(0);
        assert_eq!(dist[0], Some(0.0));
        assert_eq!(dist[2], Some(2.0));

        let communities = rg.detect_communities(50);
        assert_eq!(communities.len(), 3);
    }

    #[test]
    fn test_room_graph_blocked_routes() {
        // In an undirected graph, a Neg edge creates a 2-cycle, so use
        // directed mode for one-way blocks.
        // Layout: a(0) → b(1) → c(2), a(0) → c(2) blocked, c(2) → a(0) allowed
        // Path a→b→c = +2, a→c = -1 (negative is shorter, so shortest is -1)
        // c→a = +1 (reverse is unblocked)
        let mut g = TernaryGraph::new(3, true);
        g.add_edge(0, 1, Ternary::Positive);
        g.add_edge(1, 0, Ternary::Positive);
        g.add_edge(1, 2, Ternary::Positive);
        g.add_edge(2, 1, Ternary::Positive);
        g.add_edge(2, 0, Ternary::Positive);
        g.add_edge(0, 2, Ternary::Negative); // one-way: a → c is blocked

        let dist = shortest_paths(&g, 0);
        assert_eq!(dist[2], Some(-1.0)); // a→c direct = -1 (shorter than +2 via b)

        // From c: c→a = +1 (reverse is unblocked)
        let dist_rev = shortest_paths(&g, 2);
        assert_eq!(dist_rev[0], Some(1.0));
    }

    #[test]
    fn test_matrix_methods() {
        let mut g = TernaryGraph::new(2, false);
        g.add_edge(0, 1, Ternary::Positive);

        let adj = adjacency_f64(&g);
        assert_eq!(adj[0][1], 1.0);

        let deg = degree_matrix(&g);
        assert_eq!(deg[0][0], 1.0);

        let lap = laplacian(&g);
        assert_eq!(lap[0][0], 1.0);
        assert_eq!(lap[0][1], -1.0);
    }

    #[test]
    fn test_spectral_clustering_simple() {
        let mut g = TernaryGraph::new(4, false);
        // Two disconnected clusters
        g.add_edge(0, 1, Ternary::Positive);
        g.add_edge(2, 3, Ternary::Positive);

        let clusters = spectral_clustering(&g, 2);
        assert_eq!(clusters.len(), 4);
        assert_eq!(clusters[0], clusters[1]);
        assert_eq!(clusters[2], clusters[3]);
    }

    #[test]
    fn test_modularity() {
        let mut g = TernaryGraph::new(4, false);
        g.add_edge(0, 1, Ternary::Positive);
        g.add_edge(2, 3, Ternary::Positive);

        let perfect = vec![0, 0, 1, 1];
        let q = modularity(&g, &perfect);
        assert!(
            q > 0.0,
            "Good partition should have positive modularity, got {q}"
        );
    }

    #[test]
    fn test_ternary_re_export_type() {
        // Verify we can dereference the ternary-types Ternary directly
        let t = ternary_types::Ternary::Positive;
        assert_eq!(i8::from(t), 1);
    }
}
