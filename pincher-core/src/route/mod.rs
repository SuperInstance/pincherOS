//! # Room Routing Engine
//!
//! The `RoomGraph` gives pincher **actual pathfinding** — rooms aren't just
//! piles of data anymore. This module models room-to-room relationships as a
//! **ternary-weighted graph** where edges carry three possible weights:
//!
//! - [`Ternary::Pos`] (+1): trusted connection, preferred routing path
//! - [`Ternary::Neg`] (−1): adversarial or blocked path, avoid routing through
//! - [`Ternary::Zero`] (0): no relationship, neutral
//!
//! With this model, pincher can:
//!
//! - **Find shortest paths** through the room mesh while avoiding blocked routes
//! - **Detect communities** — rooms that naturally cluster together (via label propagation or spectral clustering)
//! - **Score partition quality** with signed modularity
//! - **Discover trusted subgraphs** — connected components over positive edges only
//! - **Compute next-hop routing** for multi-hop message delivery
//!
//! The underlying [`ternary_graph`] crate is a zero-dependency, `#![forbid(unsafe_code)]`
//! pure-Rust implementation of signed graph algorithms. This module wraps it into
//! a `RoomGraph` type that speaks pincher's domain language.

use ternary_graph::{
    all_pairs_shortest_paths, connected_components, label_propagation, modularity, shortest_paths,
    spectral_clustering, Ternary, TernaryGraph,
};

// ── Data Types ──────────────────────────────────────────────────────

/// A named node in the routing graph.
///
/// Each `Room` represents an addressable space in the pincher mesh — it could be
/// a physical room, a virtual partition, or a logical agent group.
#[derive(Clone, Debug)]
pub struct Room {
    pub id: usize,
    pub name: String,
    pub agents: Vec<String>,
}

/// The top-level routing graph.
///
/// The `RoomGraph` gives pincher actual pathfinding — rooms aren't just piles
/// of data anymore. It wraps a [`TernaryGraph`] with room metadata so routes
/// can be queried by name or id.
///
/// ## Example
///
/// ```rust,ignore
/// use pincher_core::route::build_routing_graph;
///
/// let mut g = build_routing_graph(&["lobby", "dev", "staging"]);
/// g.add_trusted_route(0, 1);
/// g.add_trusted_route(1, 2);
///
/// let dist = g.distances_from(0);
/// assert_eq!(dist[1], Some(1.0)); // lobby → dev: direct
/// assert_eq!(dist[2], Some(2.0)); // lobby → staging: via dev
/// ```
#[derive(Clone, Debug)]
pub struct RoomGraph {
    pub graph: TernaryGraph,
    pub rooms: Vec<Room>,
}

impl RoomGraph {
    /// Create a new room graph from a set of rooms.
    ///
    /// The underlying graph is built as **undirected** by default. Use
    /// [`RoomGraph::into_directed`] if you need directional edges.
    pub fn new(rooms: Vec<Room>) -> Self {
        let n = rooms.len();
        RoomGraph {
            graph: TernaryGraph::new(n, false),
            rooms,
        }
    }

    /// Convert the underlying graph to directed mode.
    ///
    /// Once directed, edges are one-way and negative-edge routing avoids
    /// the automatic negative-cycle problem inherent in undirected graphs.
    pub fn into_directed(mut self) -> Self {
        self.graph.directed = true;
        self
    }

    /// Add a trusted (positive-weight) route between two rooms.
    ///
    /// Trusted routes are preferred for message delivery, pathfinding,
    /// and community formation.
    pub fn add_trusted_route(&mut self, a: usize, b: usize) {
        self.graph.add_edge(a, b, Ternary::Pos);
    }

    /// Add a blocked (negative-weight) route between two rooms.
    ///
    /// Blocked routes represent adversarial connections, firewall rules,
    /// or rooms that should not route through each other.
    ///
    /// **Note:** In undirected graphs, a single `Neg` edge creates a
    /// two-cycle of cost −2, which Bellman-Ford correctly marks as a
    /// negative cycle. For truly one-way blocks, use [`RoomGraph::into_directed`].
    pub fn add_blocked_route(&mut self, a: usize, b: usize) {
        self.graph.add_edge(a, b, Ternary::Neg);
    }

    // ── Shortest Paths ──────────────────────────────────────────────

    /// Compute shortest paths from `source` to all other rooms using
    /// the Bellman-Ford algorithm.
    ///
    /// Rooms reachable through a negative cycle (e.g., an undirected Neg edge)
    /// return `None` — the algorithm correctly marks them as unbounded.
    ///
    /// ## Complexity
    ///
    /// O(V·E) — Bellman-Ford handles negative edge weights natively.
    pub fn distances_from(&self, source: usize) -> Vec<Option<f64>> {
        shortest_paths(&self.graph, source)
    }

    /// Find the shortest-path cost from `source` to `target`.
    ///
    /// Returns `Some(cost)` if a path exists, `None` if unreachable or
    /// affected by a negative cycle.
    pub fn route_cost(&self, source: usize, target: usize) -> Option<f64> {
        let dist = self.distances_from(source);
        dist.get(target).copied().flatten()
    }

    /// Find the cheapest next hop from `source` toward `target`.
    ///
    /// Uses all-pairs shortest paths internally. For repeated queries,
    /// consider caching the all-pairs matrix externally.
    ///
    /// Returns `Some((neighbor_id, distance_via_neighbor))` if a valid
    /// next hop exists, or `None` if `target` is unreachable.
    pub fn next_hop(&self, source: usize, target: usize) -> Option<(usize, f64)> {
        let apsp = all_pairs_shortest_paths(&self.graph);
        let source_dist = &apsp[source];

        // A valid next hop is a neighbor whose distance to target is strictly
        // less than source's distance — it makes forward progress.
        source_dist[target].and_then(|d_target| {
            self.graph
                .neighbors(source)
                .iter()
                .filter_map(|&(neighbor, _)| {
                    source_dist[neighbor].and_then(|d_n| {
                        if d_n < d_target {
                            Some((neighbor, d_n))
                        } else {
                            None
                        }
                    })
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        })
    }

    /// Compute all-pairs shortest paths for the entire graph.
    ///
    /// Uses Floyd-Warshall internally (O(V³)). Cache the result if you
    /// need multiple path queries against a static graph.
    pub fn all_distances(&self) -> Vec<Vec<Option<f64>>> {
        all_pairs_shortest_paths(&self.graph)
    }

    // ── Community Detection ──────────────────────────────────────────

    /// Detect room communities using label propagation.
    ///
    /// Positive edges act as attractive forces (rooms want to be in the same
    /// community), while negative edges act as repulsive forces.
    ///
    /// `max_iters` caps the number of label-propagation iterations.
    pub fn detect_communities(&self, max_iters: usize) -> Vec<usize> {
        label_propagation(&self.graph, max_iters)
    }

    /// Cluster rooms into `k` groups via spectral clustering.
    ///
    /// Uses power iteration on the signed Laplacian to find a low-dimensional
    /// embedding, then k-means to partition the rooms.
    pub fn cluster_rooms(&self, k: usize) -> Vec<usize> {
        spectral_clustering(&self.graph, k)
    }

    /// Compute the signed modularity score of a given community assignment.
    ///
    /// Higher values indicate stronger community structure. Use this to
    /// compare different partitions or tune clustering parameters.
    pub fn community_modularity(&self, communities: &[usize]) -> f64 {
        modularity(&self.graph, communities)
    }

    // ── Topology Analysis ────────────────────────────────────────────

    /// Find connected components using only trusted (positive) edges.
    ///
    /// This reveals which rooms form a fully-trusted subgraph — useful for
    /// identifying isolated zones or verifying reachability under trust-only
    /// routing policies.
    pub fn trusted_components(&self) -> Vec<usize> {
        connected_components(&self.graph)
    }

    /// Return the degree of a given room (total incident edges).
    pub fn degree(&self, room: usize) -> usize {
        self.graph.degree(room)
    }

    /// Return the Laplacian matrix (D − A) as a dense f64 matrix.
    pub fn laplacian(&self) -> Vec<Vec<f64>> {
        self.graph.laplacian()
    }

    /// Return the normalized Laplacian matrix.
    pub fn normalized_laplacian(&self) -> Vec<Vec<f64>> {
        self.graph.normalized_laplacian()
    }

    /// Return the adjacency matrix as a dense f64 matrix.
    /// Positive edges are +1.0, negative −1.0, zero 0.0.
    pub fn adjacency(&self) -> Vec<Vec<f64>> {
        self.graph.adjacency_f64()
    }

    /// Return the degree matrix as a dense f64 diagonal matrix.
    pub fn degree_matrix(&self) -> Vec<Vec<f64>> {
        self.graph.degree_matrix()
    }
}

// ── Factory Functions ──────────────────────────────────────────────

/// Build a simple routing graph from a list of room names.
///
/// Each name becomes a [`Room`] with an auto-incrementing `id` and an empty
/// agent list. Useful for quick setup and testing.
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
    fn test_route_basics() {
        // Triangle: lobby ↔ dev ↔ staging, plus lobby ↔ staging shortcut
        let mut rg = build_routing_graph(&["lobby", "dev", "staging"]);
        rg.add_trusted_route(0, 1);
        rg.add_trusted_route(1, 2);
        rg.add_trusted_route(0, 2);

        let dist = rg.distances_from(0);
        assert_eq!(dist[0], Some(0.0));
        assert_eq!(dist[1], Some(1.0));
        assert_eq!(dist[2], Some(1.0)); // direct edge

        let communities = rg.detect_communities(100);
        assert_eq!(communities.len(), 3);
    }

    #[test]
    fn test_route_with_blocked() {
        // Directed graph: lobby ↔ dev ↔ staging, dev → quarantine blocked
        let mut rg = build_routing_graph(&["lobby", "dev", "staging", "quarantine"]);
        // Switch to directed to avoid automatic negative cycles
        let mut g = TernaryGraph::new(4, true);
        g.add_edge(0, 1, Ternary::Pos);
        g.add_edge(1, 0, Ternary::Pos);
        g.add_edge(1, 2, Ternary::Pos);
        g.add_edge(2, 1, Ternary::Pos);
        g.add_edge(1, 3, Ternary::Neg); // one-way adversarial
        rg.graph = g;

        let dist = rg.distances_from(3);
        assert_eq!(dist[3], Some(0.0));
        assert_eq!(dist[1], None); // unreachable from quarantine

        let components = rg.trusted_components();
        assert_eq!(components[0], components[1]);
        assert_eq!(components[1], components[2]);
        assert_ne!(components[1], components[3]);
    }

    #[test]
    fn test_detect_communities_with_conflict() {
        // Two friendly groups connected by a negative edge bridge
        let mut rg = build_routing_graph(&["a1", "a2", "b1", "b2"]);
        rg.add_trusted_route(0, 1); // a-cluster
        rg.add_trusted_route(2, 3); // b-cluster
        rg.add_blocked_route(1, 2); // adversarial bridge

        let communities = rg.detect_communities(100);
        assert_eq!(
            communities[0], communities[1],
            "a1 and a2 should share a community"
        );
        assert_eq!(
            communities[2], communities[3],
            "b1 and b2 should share a community"
        );
    }

    #[test]
    fn test_spectral_routing() {
        // Two dense clusters bridged by a single trusted edge
        let mut rg =
            build_routing_graph(&["lobby", "dev", "staging", "prod", "monitor", "backup"]);
        // Cluster 1: lobby ↔ dev ↔ staging (fully connected)
        rg.add_trusted_route(0, 1);
        rg.add_trusted_route(1, 2);
        rg.add_trusted_route(0, 2);
        // Cluster 2: prod ↔ monitor ↔ backup (fully connected)
        rg.add_trusted_route(3, 4);
        rg.add_trusted_route(4, 5);
        rg.add_trusted_route(3, 5);
        // Single bridge
        rg.add_trusted_route(2, 3);

        let clusters = rg.cluster_rooms(2);
        assert_eq!(clusters.len(), 6);
        assert_eq!(clusters[0], clusters[1], "lobby and dev in same cluster");
        assert_eq!(
            clusters[0], clusters[2],
            "lobby and staging in same cluster"
        );
        assert_eq!(clusters[3], clusters[4], "prod and monitor in same cluster");
        assert_eq!(
            clusters[3], clusters[5],
            "prod and backup in same cluster"
        );
    }

    #[test]
    fn test_modularity_quality() {
        let mut rg = build_routing_graph(&["a1", "a2", "b1", "b2"]);
        rg.add_trusted_route(0, 1);
        rg.add_trusted_route(2, 3);

        // Perfect partition: {a1, a2} and {b1, b2}
        let perfect = vec![0, 0, 1, 1];
        let q_perfect = rg.community_modularity(&perfect);
        assert!(
            q_perfect > 0.0,
            "Good partition should have positive modularity"
        );

        // Bad partition: {a1, b1} and {a2, b2}
        let bad = vec![0, 1, 0, 1];
        let q_bad = rg.community_modularity(&bad);
        assert!(q_bad < q_perfect, "Bad partition should score lower");
    }

    #[test]
    fn test_route_cost() {
        let mut rg = build_routing_graph(&["a", "b", "c", "d"]);
        rg.add_trusted_route(0, 1);
        rg.add_trusted_route(1, 2);
        rg.add_trusted_route(2, 3);

        // a → d: a→b→c→d = 3 hops
        assert_eq!(rg.route_cost(0, 3), Some(3.0));
        // d → a: d→c→b→a = 3 hops
        assert_eq!(rg.route_cost(3, 0), Some(3.0));
    }

    #[test]
    fn test_next_hop_routing() {
        let mut rg = build_routing_graph(&["a", "b", "c", "d"]);
        rg.add_trusted_route(0, 1);
        rg.add_trusted_route(1, 2);
        rg.add_trusted_route(2, 3);

        // From a to d: next hop should be b (cost 1 < dist to d = 3)
        let hop = rg.next_hop(0, 3);
        assert!(hop.is_some());
        assert_eq!(hop.unwrap().0, 1); // goes to b first
    }

    #[test]
    fn test_all_distances() {
        let mut rg = build_routing_graph(&["a", "b", "c"]);
        rg.add_trusted_route(0, 1);
        rg.add_trusted_route(1, 2);

        let apsp = rg.all_distances();
        assert_eq!(apsp[0][0], Some(0.0));
        assert_eq!(apsp[0][1], Some(1.0));
        assert_eq!(apsp[0][2], Some(2.0));
        assert_eq!(apsp[2][0], Some(2.0));
    }

    #[test]
    fn test_matrix_methods() {
        let mut rg = build_routing_graph(&["a", "b"]);
        rg.add_trusted_route(0, 1);

        let adj = rg.adjacency();
        assert_eq!(adj[0][1], 1.0);
        assert_eq!(adj[1][0], 1.0);

        let deg = rg.degree_matrix();
        assert_eq!(deg[0][0], 1.0);
        assert_eq!(deg[1][1], 1.0);

        let lap = rg.laplacian();
        // L = D - A: diagonal = degree, off-diagonal = -A
        assert_eq!(lap[0][0], 1.0);
        assert_eq!(lap[0][1], -1.0);

        let nl = rg.normalized_laplacian();
        assert_eq!(nl.len(), 2);
    }

    #[test]
    fn test_degree() {
        let mut rg = build_routing_graph(&["center", "east", "west"]);
        rg.add_trusted_route(0, 1);
        rg.add_trusted_route(0, 2);

        assert_eq!(rg.degree(0), 2);
        assert_eq!(rg.degree(1), 1);
        assert_eq!(rg.degree(2), 1);
    }
}
