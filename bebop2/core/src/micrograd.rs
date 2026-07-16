//! Minimal scalar autograd engine (micrograd-style) for `bebop2-core`.
//!
//! `Value` is a node in a dynamically-built computation graph: `data` (its value),
//! `grad` (accumulated gradient w.r.t. a scalar loss), and a `backward` closure that,
//! given this node's `grad`, distributes it to its operands (`prev`). `backward()` does
//! a reverse topological sort and runs every node's closure, implementing reverse-mode
//! automatic differentiation (backprop).
//!
//! Ops: `add`, `sub`, `mul`, `pow` (constant exponent), `tanh`, `relu`, plus `neg` /
//! `scale` helpers. A tiny 2-layer MLP trained by SGD on `y = sin(x)` lives in the same
//! module and proves end-to-end gradient flow (loss drops >10x).
//!
//! Pure `std`, deterministic (caller supplies a seeded RNG). No external dependencies.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Strong handle to a graph node. Cloning shares the same underlying node — this is
/// essential so that every use of a `Value` in the graph refers to the *same* parameter
/// (e.g. a weight appears in many loss terms and must accumulate one global gradient).
type Node = Rc<RefCell<Inner>>;

/// The payload of a single graph node.
struct Inner {
    data: f64,
    grad: f64,
    /// Operands this node was built from (the edges of the DAG point result -> operands).
    prev: Vec<Node>,
    /// Reverse-mode step: given this node's `grad` (already accumulated), push the
    /// appropriate share into each operand's `grad`. Captures the operands only — NOT
    /// this node — so there is no `Rc` cycle and the graph frees cleanly.
    backward: Box<dyn Fn(&Node)>,
}

/// A differentiable scalar. Use the operator methods to build a computation graph, then
/// call [`Value::backward`] on the scalar loss to populate `.grad()` on every node.
#[derive(Clone)]
pub struct Value {
    node: Node,
}

impl Value {
    /// A fresh leaf node with value `data` and a no-op backward (it has no operands).
    pub fn new(data: f64) -> Self {
        Value {
            node: Rc::new(RefCell::new(Inner {
                data,
                grad: 0.0,
                prev: Vec::new(),
                backward: Box::new(|_| {}),
            })),
        }
    }

    pub fn data(&self) -> f64 {
        self.node.borrow().data
    }
    pub fn grad(&self) -> f64 {
        self.node.borrow().grad
    }
    /// In-place parameter update (used by SGD).
    pub fn set_data(&self, v: f64) {
        self.node.borrow_mut().data = v;
    }

    pub fn add(&self, other: &Value) -> Value {
        let a = self.node.clone();
        let b = other.node.clone();
        let data = self.data() + other.data();
        Value {
            node: Rc::new(RefCell::new(Inner {
                data,
                grad: 0.0,
                prev: vec![a.clone(), b.clone()],
                backward: Box::new(move |s: &Node| {
                    let g = s.borrow().grad;
                    a.borrow_mut().grad += g;
                    b.borrow_mut().grad += g;
                }),
            })),
        }
    }

    pub fn sub(&self, other: &Value) -> Value {
        let a = self.node.clone();
        let b = other.node.clone();
        let data = self.data() - other.data();
        Value {
            node: Rc::new(RefCell::new(Inner {
                data,
                grad: 0.0,
                prev: vec![a.clone(), b.clone()],
                backward: Box::new(move |s: &Node| {
                    let g = s.borrow().grad;
                    a.borrow_mut().grad += g;
                    b.borrow_mut().grad -= g;
                }),
            })),
        }
    }

    pub fn mul(&self, other: &Value) -> Value {
        let a = self.node.clone();
        let b = other.node.clone();
        let data = self.data() * other.data();
        Value {
            node: Rc::new(RefCell::new(Inner {
                data,
                grad: 0.0,
                prev: vec![a.clone(), b.clone()],
                backward: Box::new(move |s: &Node| {
                    let g = s.borrow().grad;
                    let a_data = a.borrow().data;
                    let b_data = b.borrow().data;
                    a.borrow_mut().grad += g * b_data;
                    b.borrow_mut().grad += g * a_data;
                }),
            })),
        }
    }

    /// Raise to a *constant* power `n`. Backward: `d(a^n)/da = n * a^(n-1)`.
    pub fn pow(&self, n: f64) -> Value {
        let a = self.node.clone();
        let data = self.data().powf(n);
        Value {
            node: Rc::new(RefCell::new(Inner {
                data,
                grad: 0.0,
                prev: vec![a.clone()],
                backward: Box::new(move |s: &Node| {
                    let g = s.borrow().grad;
                    let a_data = a.borrow().data;
                    a.borrow_mut().grad += g * n * a_data.powf(n - 1.0);
                }),
            })),
        }
    }

    pub fn tanh(&self) -> Value {
        let a = self.node.clone();
        let t = self.data().tanh();
        Value {
            node: Rc::new(RefCell::new(Inner {
                data: t,
                grad: 0.0,
                prev: vec![a.clone()],
                backward: Box::new(move |s: &Node| {
                    let g = s.borrow().grad;
                    let t = s.borrow().data;
                    a.borrow_mut().grad += g * (1.0 - t * t);
                }),
            })),
        }
    }

    pub fn relu(&self) -> Value {
        let a = self.node.clone();
        let data = if self.data() > 0.0 { self.data() } else { 0.0 };
        Value {
            node: Rc::new(RefCell::new(Inner {
                data,
                grad: 0.0,
                prev: vec![a.clone()],
                backward: Box::new(move |s: &Node| {
                    let g = s.borrow().grad;
                    let a_data = a.borrow().data;
                    if a_data > 0.0 {
                        a.borrow_mut().grad += g;
                    }
                }),
            })),
        }
    }

    /// `-self` via multiplication by a constant `-1`.
    pub fn neg(&self) -> Value {
        self.mul(&Value::new(-1.0))
    }

    /// `self * k` via multiplication by a constant.
    pub fn scale(&self, k: f64) -> Value {
        self.mul(&Value::new(k))
    }

    /// Reverse-mode automatic differentiation. Seeds this node's gradient to `1.0` and
    /// propagates backward through the graph in reverse topological order, so each node's
    /// `grad` is final before it distributes into its operands.
    pub fn backward(&self) {
        // Post-order DFS gives operands before results; reversing yields root-first order.
        let mut topo: Vec<Node> = Vec::new();
        let mut visited: HashSet<*const RefCell<Inner>> = HashSet::new();
        build_topo(&self.node, &mut topo, &mut visited);

        // Zero all grads, then seed the root (this node).
        for n in &topo {
            n.borrow_mut().grad = 0.0;
        }
        self.node.borrow_mut().grad = 1.0;

        for n in topo.iter().rev() {
            let inner = n.borrow();
            // `inner` is an immutable borrow of `n`; the closure reads `n` (immutable)
            // and mutates only the operand nodes — no aliasing of `n`.
            (inner.backward)(n);
        }
    }
}

/// Post-order (operands-before-result) topological traversal by node identity.
fn build_topo(node: &Node, topo: &mut Vec<Node>, visited: &mut HashSet<*const RefCell<Inner>>) {
    let ptr = Rc::as_ptr(node);
    if visited.insert(ptr) {
        for child in &node.borrow().prev {
            build_topo(child, topo, visited);
        }
        topo.push(node.clone());
    }
}

/// Tiny deterministic PRNG (xorshift64*) so training is reproducible with a fixed seed.
/// Pure-std, no system entropy.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid a zero state which would wedge xorshift.
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// Uniform in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// A feed-forward MLP with `tanh` hidden layers and a linear output. Parameters are
/// exposed as `Value`s so they participate directly in the autograd graph.
pub struct Mlp {
    layers: Vec<Layer>,
    params: Vec<Value>,
}

struct Layer {
    nin: usize,
    nout: usize,
    w: Vec<Vec<Value>>, // [nin][nout]
    b: Vec<Value>,      // [nout]
}

impl Mlp {
    /// Build an MLP with layer sizes `sizes = [input, hidden…, output]`. Weights are
    /// initialised as `U(-1,1) / sqrt(fan_in)` (Xavier-ish), biases to zero.
    pub fn new(sizes: &[usize], rng: &mut Rng) -> Self {
        assert!(sizes.len() >= 2, "need at least input+output");
        let mut layers = Vec::new();
        let mut params = Vec::new();
        for w in sizes.windows(2) {
            let nin = w[0];
            let nout = w[1];
            let scale = 1.0 / (nin as f64).sqrt();
            let mut weight = Vec::with_capacity(nin);
            for _ in 0..nin {
                let mut row = Vec::with_capacity(nout);
                for _ in 0..nout {
                    let v = Value::new((rng.next_f64() * 2.0 - 1.0) * scale);
                    params.push(v.clone());
                    row.push(v);
                }
                weight.push(row);
            }
            let mut bias = Vec::with_capacity(nout);
            for _ in 0..nout {
                let v = Value::new(0.0);
                params.push(v.clone());
                bias.push(v);
            }
            layers.push(Layer {
                nin,
                nout,
                w: weight,
                b: bias,
            });
        }
        Mlp { layers, params }
    }

    /// Forward pass. Hidden layers use `tanh`; the final layer is linear (so it can match
    /// `sin(x)` whose range is `[-1, 1]` exactly).
    pub fn forward(&self, xs: &[Value]) -> Vec<Value> {
        let last = self.layers.len() - 1;
        let mut h = xs.to_vec();
        for (li, layer) in self.layers.iter().enumerate() {
            let mut out = Vec::with_capacity(layer.nout);
            for j in 0..layer.nout {
                let mut s = layer.b[j].clone();
                for i in 0..layer.nin {
                    s = s.add(&layer.w[i][j].mul(&h[i]));
                }
                out.push(if li < last { s.tanh() } else { s });
            }
            h = out;
        }
        h
    }

    /// All trainable parameters (shared `Rc`s with those used in `forward`).
    pub fn params(&self) -> &[Value] {
        &self.params
    }

    /// Train by full-batch SGD on mean-squared error. Returns per-step loss so the caller
    /// can assert the loss drops. `data` is `(x, y)` pairs.
    pub fn train(&mut self, data: &[(f64, f64)], lr: f64, steps: usize) -> Vec<f64> {
        let n = data.len() as f64;
        let mut losses = Vec::with_capacity(steps);
        for _ in 0..steps {
            // Total loss = mean over the batch of (yhat - y)^2.
            let mut total = Value::new(0.0);
            for &(x, y) in data {
                let out = self.forward(&[Value::new(x)])[0].clone();
                let diff = out.sub(&Value::new(y));
                total = total.add(&diff.mul(&diff));
            }
            total = total.scale(1.0 / n);
            losses.push(total.data());
            total.backward();
            // SGD: w <- w - lr * dw.
            for p in &self.params {
                let g = p.grad();
                p.set_data(p.data() - lr * g);
            }
        }
        losses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Analytic vs finite-difference gradient of a scalar function of one variable.
    fn numeric_grad(f: impl Fn(f64) -> f64, x: f64, eps: f64) -> f64 {
        (f(x + eps) - f(x - eps)) / (2.0 * eps)
    }

    #[test]
    fn micrograd_gradient_xy_at_2_3() {
        // RED->GREEN: f = x*y, analytic gradient (df/dx, df/dy) = (y, x) = (3, 2).
        let x = Value::new(2.0);
        let y = Value::new(3.0);
        let f = x.mul(&y);
        f.backward();
        assert!(
            (x.grad() - 3.0).abs() < 1e-12,
            "df/dx = {}, expected 3",
            x.grad()
        );
        assert!(
            (y.grad() - 2.0).abs() < 1e-12,
            "df/dy = {}, expected 2",
            y.grad()
        );
    }

    #[test]
    fn micrograd_tanh_finite_diff() {
        // RED->GREEN: tanh backward must match central finite differences.
        let eps = 1e-6;
        for a in [-2.0_f64, -0.7, 0.0, 0.4, 1.3, 2.5] {
            let v = Value::new(a);
            let t = v.tanh();
            t.backward();
            let analytic = v.grad();
            let numeric = numeric_grad(|x| x.tanh(), a, eps);
            assert!(
                (analytic - numeric).abs() < 1e-5,
                "tanh grad at {a}: analytic={analytic}, numeric={numeric}"
            );
            // Also check the closed form 1 - tanh^2.
            let closed = 1.0 - t.data() * t.data();
            assert!((analytic - closed).abs() < 1e-12);
        }
    }

    #[test]
    fn micrograd_composite_finite_diff() {
        // A composite of mul/add/pow/tanh/relu; compare autograd to finite differences
        // for both inputs. Verifies the whole op set wires together correctly.
        let eps = 1e-6;
        for (x0, y0) in [(1.0_f64, 2.0), (-1.5, 0.5), (0.7, -2.0)] {
            // f(x, y) = tanh((x*y + x^3) ) + relu(y - 1)  -- a mixed expression.
            let x = Value::new(x0);
            let y = Value::new(y0);
            let inner = x.mul(&y).add(&x.pow(3.0));
            let f = inner.tanh().add(&y.sub(&Value::new(1.0)).relu());
            f.backward();

            let gx = x.grad();
            let gy = y.grad();
            let fx = |xv: f64| -> f64 {
                let i = xv * y0 + xv.powi(3);
                i.tanh() + (y0 - 1.0).max(0.0)
            };
            let fy = |yv: f64| -> f64 {
                let i = x0 * yv + x0.powi(3);
                i.tanh() + (yv - 1.0).max(0.0)
            };
            let nx = numeric_grad(fx, x0, eps);
            let ny = numeric_grad(fy, y0, eps);
            assert!(
                (gx - nx).abs() < 1e-4,
                "f/dx at ({x0},{y0}): autograd={gx}, numeric={nx}"
            );
            assert!(
                (gy - ny).abs() < 1e-4,
                "f/dy at ({x0},{y0}): autograd={gy}, numeric={ny}"
            );
        }
    }

    #[test]
    fn micrograd_pow_gradient() {
        // d(x^3)/dx at x=2 = 3*4 = 12.
        let x = Value::new(2.0);
        let f = x.pow(3.0);
        f.backward();
        assert!((x.grad() - 12.0).abs() < 1e-12, "got {}", x.grad());
    }

    #[test]
    fn micrograd_mlp_loss_drop() {
        // RED->GREEN: train a tiny 2-layer MLP on y = sin(x); loss must drop >10x.
        let mut rng = Rng::new(0xBEEF);
        // Deterministic synthetic data: 40 points x in [-3, 3], y = sin(x).
        let mut data = Vec::new();
        let n = 40usize;
        for i in 0..n {
            let x = -3.0 + 6.0 * (i as f64) / ((n - 1) as f64);
            data.push((x, x.sin()));
        }

        let mut mlp = Mlp::new(&[1, 20, 1], &mut rng);
        let losses = mlp.train(&data, 0.05, 2500);

        let initial = losses[0];
        let final_loss = *losses.last().unwrap();
        // Sample a few points for the curve summary.
        let sample_idx: Vec<usize> = (0..losses.len())
            .step_by(losses.len() / 10.max(1))
            .collect();
        eprintln!("micrograd MLP loss curve (seed=0xBEEF, [1,20,1], lr=0.05, steps=2500):");
        eprintln!("  initial = {initial:.6e}");
        for &idx in &sample_idx {
            eprintln!("  step {idx:>4}: {:.6e}", losses[idx]);
        }
        eprintln!("  final   = {final_loss:.6e}");
        eprintln!(
            "  ratio (final/initial) = {:.3e}  (must be < 0.1)",
            final_loss / initial
        );

        assert!(
            final_loss < initial / 10.0,
            "MLP loss did not drop >10x: initial={initial}, final={final_loss}"
        );
    }
}
