/**
 * matrix.test.ts — RED+GREEN falsifiable tests for the deterministic
 * linear-algebra primitives (Jacobi EVD, two-sided SVD, PCA).
 *
 * GREEN: decompositions reconstruct the input within ε (grounded in the
 *   algebraic identities A = V·diag(λ)·Vᵀ and A = U·S·Vᵀ); PCA reconstruction
 *   error is ~0 for a vector drawn from the same manifold.
 * RED:   malformed input (non-square, NaN, <2 samples, dim mismatch) is
 *   rejected — no silent garbage.
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { svd, pcaFit, pcaProject, pcaReconstruct, jacobiEVD, transpose, matmul } from './matrix.ts';

// tiny Frobenius norm helper
function fro(A: number[][]): number {
  let s = 0;
  for (const row of A) for (const v of row) s += v * v;
  return Math.sqrt(s);
}

test('GREEN: SVD reconstructs a 3×2 matrix A = U·diag(S)·Vᵀ within ε', () => {
  const A = [
    [2, 0],
    [0, 3],
    [1, 1],
  ];
  const { U, S, V } = svd(A);
  // rebuild U·diag(S)·Vᵀ
  const diag: number[][] = S.map((s, i) => S.map((_, j) => (i === j ? s : 0)));
  const US = matmul(U, diag);
  const VT = transpose(V);
  const Ahat = matmul(US, VT);
  const err = fro(A.map((row, i) => row.map((v, j) => v - Ahat[i][j])));
  assert.ok(err < 1e-9, `SVD reconstruction error ${err} should be ~0`);
});

test('GREEN: singular values of a known matrix match the analytic norm', () => {
  // A = [[3,0],[0,4]] → singular values {4,3}
  const A = [
    [3, 0],
    [0, 4],
  ];
  const { S } = svd(A);
  assert.ok(Math.abs(S[0] - 4) < 1e-9, `top singular value ${S[0]}`);
  assert.ok(Math.abs(S[1] - 3) < 1e-9, `second singular value ${S[1]}`);
});

test('GREEN: Jacobi EVD of a symmetric matrix reconstructs A = V·Λ·Vᵀ', () => {
  const A = [
    [2, 1],
    [1, 2],
  ];
  const { values, vectors } = jacobiEVD(A);
  const diag = values.map((l, i) => values.map((_, j) => (i === j ? l : 0)));
  const VLVt = matmul(matmul(vectors, diag), transpose(vectors));
  const err = fro(A.map((row, i) => row.map((v, j) => v - VLVt[i][j])));
  assert.ok(err < 1e-9, `EVD reconstruction error ${err}`);
  // eigenvalues of [[2,1],[1,2]] are 3 and 1
  const sorted = [...values].sort((a, b) => b - a);
  assert.ok(Math.abs(sorted[0] - 3) < 1e-9 && Math.abs(sorted[1] - 1) < 1e-9, `eigs ${values}`);
});

test('GREEN: PCA reconstruction of an in-manifold sample is near-exact', () => {
  // build a 2D-line manifold: x = [a, 2a+1] for varying a
  const X: number[][] = [];
  for (let a = -5; a <= 5; a += 0.5) X.push([a, 2 * a + 1]);
  const pca = pcaFit(X);
  const z = pcaProject(pca, [3, 7]); // lies on the line
  const xhat = pcaReconstruct(pca, z);
  const err = Math.hypot(xhat[0] - 3, xhat[1] - 7);
  assert.ok(err < 1e-9, `in-manifold reconstruction error ${err}`);
});

test('GREEN: PCA top component captures the dominant variance direction', () => {
  // data spread mostly along x (var 100) vs y (var 1) → axis[0] ≈ [1,0]
  const X: number[][] = [];
  for (let i = 0; i < 40; i++) X.push([(i - 20) * 0.5, (Math.round(Math.sin(i) * 10)) / 10]);
  const pca = pcaFit(X);
  const axis = pca.components[0];
  // dominant axis should load far more on x than y
  assert.ok(Math.abs(axis[0]) > 2 * Math.abs(axis[1]), `axis ${axis}`);
  // first explained variance should dominate
  assert.ok(pca.explainedVariance[0] > pca.explainedVariance[1], 'variance[0] > variance[1]');
});

// ── RED cases (must FAIL on bad input) ──

test('RED: SVD of a non-rectangular (ragged) matrix throws', () => {
  assert.throws(() => svd([[1, 2], [3]]));
});

test('RED: SVD of a non-finite matrix throws', () => {
  assert.throws(() => svd([[1, 2], [3, NaN]]));
});

test('RED: EVD of a non-square matrix throws', () => {
  assert.throws(() => jacobiEVD([[1, 2, 3], [4, 5, 6]]));
});

test('RED: PCA with <2 samples throws', () => {
  assert.throws(() => pcaFit([[1, 2, 3]]));
});

test('RED: pcaProject with dim mismatch throws', () => {
  const X: number[][] = [
    [1, 2],
    [3, 4],
  ];
  const pca = pcaFit(X);
  assert.throws(() => pcaProject(pca, [1]));
});
