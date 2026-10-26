//! HOW THICK A BODY'S WALLS ARE: points spread over its surface by area, from each a ray into the material against
//! the face's outward normal, and the distance to the first face it meets - the wall there. A wall thinner than two
//! lines of the nozzle prints as one line or none. The rays run through a tree of boxes over the triangles, so a mesh
//! of a million triangles is read in the time of the points, not of points times triangles.

use crate::print::checks::Verdict;
use qymcad_core::geom::Mesh;

/// What a reading of the walls is asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Walls {
    /// The thinnest wall that prints, mm: two lines of the nozzle.
    pub least: f64,
    /// How many points are read over the surface, spread by area.
    pub samples: usize,
}

/// A place a wall is thinner than asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thin {
    /// The point on the surface, in the frame of the mesh.
    pub at: [f64; 3],
    pub thickness: f64,
}

/// How thick the walls read.
#[derive(Clone, Debug, PartialEq)]
pub struct Thickness {
    pub verdict: Verdict,
    /// The thinnest wall read, mm; infinity on a mesh no ray crossed.
    pub least: f64,
    /// The places thinner than asked, the thinnest first, at least `SPREAD` of the box's diagonal apart; at most
    /// `crate::print::checks::PLACES`.
    pub places: Vec<Thin>,
    /// How many points were read.
    pub samples: usize,
}

/// How far apart two places told are, as a share of the diagonal of the body's box: closer, they are one place.
const SPREAD: f64 = 0.05;

/// The walls of `mesh` as `walls` asks to read them.
pub fn thickness(mesh: &Mesh, walls: &Walls) -> Thickness {
    let tree = Tree::of(mesh);
    let diagonal = mesh.bounds().map_or(0.0, |b| ((b.max.x - b.min.x).powi(2) + (b.max.y - b.min.y).powi(2) + (b.max.z - b.min.z).powi(2)).sqrt());
    // a ray starts this far inside its face and passes over hits nearer: the face itself, met again by rounding
    let lead = (diagonal * 1e-9).max(1e-12);
    let mut read: Vec<Thin> = Vec::new();
    for s in spread(mesh, walls.samples) {
        let (n, _, _) = mesh.tri_normal_area(s.tri);
        let dir = [-n[0], -n[1], -n[2]];
        let from = [s.at[0] + dir[0] * lead, s.at[1] + dir[1] * lead, s.at[2] + dir[2] * lead];
        if let Some(d) = tree.first_hit(mesh, &Ray { from, dir }, s.tri) {
            read.push(Thin { at: s.at, thickness: d + lead });
        }
    }
    let least = read.iter().map(|t| t.thickness).fold(f64::INFINITY, f64::min);
    let mut thin: Vec<Thin> = read.iter().copied().filter(|t| t.thickness < walls.least).collect();
    thin.sort_by(|a, b| a.thickness.total_cmp(&b.thickness));
    let apart = diagonal * SPREAD;
    let mut places: Vec<Thin> = Vec::new();
    for t in thin {
        if places.len() >= crate::print::checks::PLACES {
            break;
        }
        let far = |p: &Thin| (0..3).map(|k| (p.at[k] - t.at[k]).powi(2)).sum::<f64>().sqrt() >= apart;
        if places.iter().all(far) {
            places.push(t);
        }
    }
    let verdict = if least < walls.least { Verdict::Fail } else { Verdict::Pass };
    Thickness { verdict, least, places, samples: read.len() }
}

/// A point on the surface: the triangle it lies on and where.
struct Sample {
    tri: usize,
    at: [f64; 3],
}

/// About `count` points over the triangles of `mesh`, spread by area: the k-th at k + 1/2 shares of the area along the
/// triangles in their order, placed within its triangle by a sequence that fills a triangle evenly. The same mesh gives
/// the same points.
fn spread(mesh: &Mesh, count: usize) -> Vec<Sample> {
    let areas: Vec<f64> = (0..mesh.tris.len()).map(|t| mesh.tri_normal_area(t).1).collect();
    let total: f64 = areas.iter().sum();
    if total <= 0.0 || count == 0 {
        return Vec::new();
    }
    let step = total / count as f64;
    let mut out = Vec::with_capacity(count);
    let mut next = step / 2.0;
    let mut run = 0.0;
    let mut k = 0usize;
    for (t, &a) in areas.iter().enumerate() {
        run += a;
        while next < run {
            // the plastic number's sequence in two dimensions, folded into the triangle
            let (mut u, mut v) = ((0.5 + 0.754_877_666 * k as f64).fract(), (0.5 + 0.569_840_291 * k as f64).fract());
            if u + v > 1.0 {
                u = 1.0 - u;
                v = 1.0 - v;
            }
            let [p, q, r] = mesh.triangle(t);
            let at = [p.x + u * (q.x - p.x) + v * (r.x - p.x), p.y + u * (q.y - p.y) + v * (r.y - p.y), p.z + u * (q.z - p.z) + v * (r.z - p.z)];
            out.push(Sample { tri: t, at });
            next += step;
            k += 1;
        }
    }
    out
}

/// A ray: where it starts and its unit direction.
struct Ray {
    from: [f64; 3],
    dir: [f64; 3],
}

/// A box of the tree and what it holds: two boxes under it, or a run of triangles.
struct Node {
    lo: [f64; 3],
    hi: [f64; 3],
    held: Held,
}

enum Held {
    Inner { left: usize, right: usize },
    Leaf(Run),
}

/// A run of the tree's triangles: where it starts in `Tree::order` and how many.
#[derive(Clone, Copy)]
struct Run {
    first: usize,
    count: usize,
}

/// THE TREE OF BOXES over the triangles: each box holds the triangles under it, split at the middle of the longest side
/// of their middles until a few are left.
struct Tree {
    nodes: Vec<Node>,
    order: Vec<usize>,
}

/// The triangles a leaf holds at most.
const LEAF: usize = 4;

impl Tree {
    fn of(mesh: &Mesh) -> Tree {
        let boxes: Vec<[[f64; 3]; 2]> = (0..mesh.tris.len())
            .map(|t| {
                let p = mesh.triangle(t);
                let lo = [p[0].x.min(p[1].x).min(p[2].x), p[0].y.min(p[1].y).min(p[2].y), p[0].z.min(p[1].z).min(p[2].z)];
                let hi = [p[0].x.max(p[1].x).max(p[2].x), p[0].y.max(p[1].y).max(p[2].y), p[0].z.max(p[1].z).max(p[2].z)];
                [lo, hi]
            })
            .collect();
        let mut tree = Tree { nodes: Vec::new(), order: (0..mesh.tris.len()).collect() };
        if !boxes.is_empty() {
            let count = tree.order.len();
            tree.build(&boxes, Run { first: 0, count });
        }
        tree
    }

    /// Lays the node over the triangles of `run`; its index.
    fn build(&mut self, boxes: &[[[f64; 3]; 2]], run: Run) -> usize {
        let Run { first, count } = run;
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for &t in &self.order[first..first + count] {
            for k in 0..3 {
                lo[k] = lo[k].min(boxes[t][0][k]);
                hi[k] = hi[k].max(boxes[t][1][k]);
            }
        }
        let me = self.nodes.len();
        self.nodes.push(Node { lo, hi, held: Held::Leaf(run) });
        if count <= LEAF {
            return me;
        }
        let axis = (0..3).max_by(|&a, &b| (hi[a] - lo[a]).total_cmp(&(hi[b] - lo[b]))).unwrap_or(0);
        let mid = |t: usize| boxes[t][0][axis] + boxes[t][1][axis];
        let half = count / 2;
        self.order[first..first + count].select_nth_unstable_by(half, |&a, &b| mid(a).total_cmp(&mid(b)));
        let left = self.build(boxes, Run { first, count: half });
        let right = self.build(boxes, Run { first: first + half, count: count - half });
        self.nodes[me].held = Held::Inner { left, right };
        me
    }

    /// The distance along `ray` to the first triangle it crosses, triangle `own` aside.
    fn first_hit(&self, mesh: &Mesh, ray: &Ray, own: usize) -> Option<f64> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = ray.dir.map(|d| if d.abs() > 1e-300 { 1.0 / d } else { f64::INFINITY.copysign(d) });
        let mut best = f64::INFINITY;
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i];
            if !enters(n, ray, &inv, best) {
                continue;
            }
            match n.held {
                Held::Inner { left, right } => {
                    stack.push(left);
                    stack.push(right);
                }
                Held::Leaf(Run { first, count }) => {
                    for &t in &self.order[first..first + count] {
                        if t == own {
                            continue;
                        }
                        if let Some(d) = crosses(mesh, t, ray).filter(|&d| d < best) {
                            best = d;
                        }
                    }
                }
            }
        }
        best.is_finite().then_some(best)
    }
}

/// Whether `ray` enters the box of `n` nearer than `within` (the slab test).
fn enters(n: &Node, ray: &Ray, inv: &[f64; 3], within: f64) -> bool {
    let mut near = 0.0f64;
    let mut far = within;
    for (((lo, hi), from), inv) in n.lo.iter().zip(&n.hi).zip(&ray.from).zip(inv) {
        let a = (lo - from) * inv;
        let b = (hi - from) * inv;
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        near = near.max(if a.is_nan() { f64::NEG_INFINITY } else { a });
        far = far.min(if b.is_nan() { f64::INFINITY } else { b });
        if near > far {
            return false;
        }
    }
    true
}

/// The distance along `ray` to triangle `t`, where it crosses it ahead of its start (Moller-Trumbore).
fn crosses(mesh: &Mesh, t: usize, ray: &Ray) -> Option<f64> {
    let [p, q, r] = mesh.triangle(t);
    let e1 = [q.x - p.x, q.y - p.y, q.z - p.z];
    let e2 = [r.x - p.x, r.y - p.y, r.z - p.z];
    let d = ray.dir;
    let h = [d[1] * e2[2] - d[2] * e2[1], d[2] * e2[0] - d[0] * e2[2], d[0] * e2[1] - d[1] * e2[0]];
    let a = e1[0] * h[0] + e1[1] * h[1] + e1[2] * h[2];
    if a.abs() < 1e-14 {
        return None;
    }
    let f = 1.0 / a;
    let s = [ray.from[0] - p.x, ray.from[1] - p.y, ray.from[2] - p.z];
    let u = f * (s[0] * h[0] + s[1] * h[1] + s[2] * h[2]);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qv = [s[1] * e1[2] - s[2] * e1[1], s[2] * e1[0] - s[0] * e1[2], s[0] * e1[1] - s[1] * e1[0]];
    let v = f * (d[0] * qv[0] + d[1] * qv[1] + d[2] * qv[2]);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let dist = f * (e2[0] * qv[0] + e2[1] * qv[1] + e2[2] * qv[2]);
    (dist > 0.0).then_some(dist)
}
